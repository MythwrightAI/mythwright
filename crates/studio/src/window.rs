//! A table read a window at a time, by the daemon.
//!
//! What a window of a view is and how it is read is decided in
//! `kernel::window` (the order, what a view shows, where a reading got
//! to, the plan), with its cases; this turns that into statements, each
//! asking the index the order is walked by by name (`INDEXED BY`) and
//! saying the project and the table as literals, which is what the index
//! of one table's column is used for (`indexes`). So a window costs the
//! database the same steps whatever the table holds: it begins at its
//! place in an index and reads on from there, and an answer that has to
//! check what rows hold to know whether the view shows them stops at
//! [`LOOK`] entries looked at and says where it got to (`short`).
//!
//! Three ways (`kernel::window::Way`): **walk** an index in the order
//! asked, a stretch at a time; **gather** the rows a view is held to by an
//! indexed column where they are few, and sort them in the statement; or
//! sort a **small** table in the statement. A table kept anywhere but the
//! database is read whole where it is kept and windowed in memory by the
//! same rules (`kernel::window::read`), and says it was.
//!
//! What is counted of a view (how many rows, in each band, the figures at
//! its columns' feet) is a second question ([`Collections::counted_rows`]):
//! read off the indexes where nothing has to be checked of the rows, and
//! otherwise worked out in pieces of [`PIECE`] rows, [`PIECES`] of them an
//! answer, each piece of work by itself so that everything else goes on
//! between them. What it came to is kept by the table's write mark
//! (`counted`), so that two pages share one counting and one that is not
//! through goes on where it got to.

use std::collections::BTreeMap;

use kernel::window::figures::{Counting, ordered_bands};
use kernel::window::found::{self, Find};
use kernel::window::read::{From, window as in_hand};
use kernel::window::statements::{self, Shape};
use kernel::window::{
    Asked as Planned, Column, Gather, Held, How, Order, PIECE, PIECES, Place, WHOLE_READ, Way, cursor, held,
    mark, place_of, plan,
};
use kernel::{Error, Principal, Result};
use serde_json::{Map, Value, json};

pub(crate) use ::windowed::Counts;
pub use ::windowed::{Answer, Asked, Reading};
use ::windowed::{off_index, piece, rank_before, read_window};

use crate::collections::{Collection, Collections, RECORD, Record, Store};
use crate::indexes::column;
use crate::records::{COMMENTED, Records};

/// What a view comes to, as the table is now.
struct Viewed {
    columns: Vec<Column>,
    held: Vec<Held>,
    find: Option<Find>,
    group: Option<Column>,
    figures: BTreeMap<String, String>,
    mark: String,
}

fn viewed(table: &Collection, asked: &Asked) -> Result<Viewed> {
    let columns: Vec<Column> = table.fields.iter().map(column).collect();
    let view =
        match &asked.view {
            None => None,
            Some(name) => {
                Some(table.views.iter().find(|view| &view.name == name).ok_or_else(|| {
                    Error::not_found(format!("`{}` has no view called `{name}`", table.name))
                })?)
            }
        };
    let only = view.and_then(|view| view.only.clone()).unwrap_or_default();
    let by = view.and_then(|view| view.group.clone());
    let band = match (&asked.band, &by) {
        (None, _) => None,
        (Some(band), Some(by)) => Some((by.as_str(), band.as_str())),
        (Some(_), None) => {
            return Err(Error::invalid(
                "a band is read of a view gathered by a column (`group`), and this one gathers by none",
            ));
        }
    };
    let held = held(&columns, &only, band);
    let hides = view.and_then(|view| view.hides.clone()).unwrap_or_default();
    let find = asked.find.as_deref().and_then(|typed| Find::of(&columns, &hides, typed));
    let group = by.and_then(|by| columns.iter().find(|column| column.name == by).cloned());
    let figures = view.and_then(|view| view.shown.figures.clone()).unwrap_or_default();
    let words = find.as_ref().map(|find| find.words.clone()).unwrap_or_default();
    let mark = mark(asked.view.as_deref().unwrap_or_default(), &held, asked.sort.as_ref(), &words);
    Ok(Viewed { columns, held, find, group, figures, mark })
}

/// A row as a window hands it: the fields its table declares (those
/// asked, where some are), and how many things were said on it.
fn handed(table: &Collection, mut record: Record, fields: Option<&[String]>, commented: u64) -> Value {
    record.data.retain(|name, _| {
        table.field(name).is_some() && fields.is_none_or(|fields| fields.iter().any(|one| one == name))
    });
    json!({
        "id": record.id, "number": record.number, "data": record.data, "position": record.position,
        "created_by": record.created_by, "created": record.created, "updated": record.updated, "commented": commented,
    })
}

impl Collections {
    /// A window of `table` as `asked`. Whoever asks has said who may read it.
    pub async fn window(&self, table: &Collection, asked: &Asked, by: &Principal) -> Result<Answer> {
        let viewed = viewed(table, asked)?;
        let limit = asked.limit.clamp(1, kernel::window::MOST);
        let from = match &asked.reading {
            Reading::Start => From::Start,
            Reading::After(text) => From::After(place_of(text, &viewed.mark)?),
            Reading::Before(text) => From::Before(place_of(text, &viewed.mark)?),
            Reading::At(rank) => From::At(*rank),
            Reading::Around(said) => return self.around(table, &viewed, asked, said, limit, by).await,
        };
        self.read(table, &viewed, asked, from, limit, by).await
    }

    /// How a window of a table kept in the database is read: the shape of
    /// its statements, and whether they walk an index or gather the rows
    /// a view is held to; the columns it goes by indexed first.
    async fn planned(
        &self,
        table: &Collection,
        viewed: &Viewed,
        asked: &Asked,
        by: &Principal,
    ) -> Result<(Shape, Way, Option<Gather>)> {
        let count = self.count(table).await? as u64;
        let indexed = self.indexed(table).await?;
        let planned = Planned {
            columns: &viewed.columns,
            held: &viewed.held,
            sort: asked.sort.as_ref(),
            count,
            indexed: &indexed,
        };
        let plan = plan(&planned)?;
        if !plan.make.is_empty() {
            self.index(table, &plan.make, by).await?;
        }
        let mut used: Vec<String> = plan.order.column.iter().map(|column| column.name.clone()).collect();
        used.extend(plan.gather.iter().map(|gather| gather.column.name.clone()));
        self.asked_by(table, &used);
        let shape = Shape {
            record: RECORD.to_string(),
            project: table.project.clone(),
            table: table.name.clone(),
            order: plan.order.clone(),
            shows: found::shows_sql(&viewed.held, viewed.find.as_ref()),
        };
        Ok((shape, plan.way, plan.gather))
    }

    /// A window read `from` where said.
    async fn read(
        &self,
        table: &Collection,
        viewed: &Viewed,
        asked: &Asked,
        from: From,
        limit: usize,
        by: &Principal,
    ) -> Result<Answer> {
        let fields = asked.fields.as_deref();
        if table.store != Store::Local {
            return self.window_in_hand(table, viewed, asked, &from, limit).await;
        }
        let (shape, way, gather) = self.planned(table, viewed, asked, by).await?;
        let read = self
            .rows(&table.project)
            .db()
            .call(move |connection| read_window(connection, &shape, way, gather, &from, limit, COMMENTED))
            .await?;
        Ok(Answer {
            rows: read
                .rows
                .into_iter()
                .map(|((record, _), said)| handed(table, record, fields, said))
                .collect(),
            after: read.after.map(|place| cursor(&viewed.mark, &place)),
            before: read.before.map(|place| cursor(&viewed.mark, &place)),
            looked: read.looked,
            short: read.short,
            rank: read.rank,
            whole_read: None,
            around: None,
        })
    }

    /// A window about one row, by its id or its number: the rows before it
    /// and from it on, the first standing where the answer says; or, where
    /// the view does not show it, what keeps it out.
    async fn around(
        &self,
        table: &Collection,
        viewed: &Viewed,
        asked: &Asked,
        said: &str,
        limit: usize,
        by: &Principal,
    ) -> Result<Answer> {
        let said = said.trim();
        let id = match said.parse::<i64>() {
            Ok(number) => crate::collections::record_id(&table.project, &table.name, number),
            Err(_) => said.to_string(),
        };
        let number = id.rsplit_once('-').and_then(|(_, number)| number.parse::<i64>().ok());
        let record = match number {
            Some(number) if table.owns(&id) => self.one(table, &id, number).await?,
            _ => None,
        };
        let record = record.ok_or_else(|| {
            Error::not_found(format!(
                "`{}` has no row `{said}`: go to a row by its id or its number",
                table.name
            ))
        })?;
        let out = found::kept_out(&viewed.held, viewed.find.as_ref(), &record.data);
        let row = handed(table, record.clone(), asked.fields.as_deref(), 0);
        if !out.is_empty() {
            let around = json!({ "id": record.id, "in": false, "kept_out": out, "row": row });
            return Ok(Answer { around: Some(around), ..Answer::default() });
        }
        let order = Order::of(&viewed.columns, asked.sort.as_ref())?;
        let place = order.place(&record.data, record.number);
        let rank = self.rank(table, viewed, asked, &order, &place, by).await?;
        let (back, on) = found::about(&place);
        let before = self.read(table, viewed, asked, back, (limit / 2).max(1), by).await?;
        let after =
            self.read(table, viewed, asked, on, limit.saturating_sub(before.rows.len()).max(1), by).await?;
        let first = rank.saturating_sub(before.rows.len() as u64);
        Ok(Answer {
            looked: before.looked + after.looked,
            short: before.short || after.short,
            rank: Some(first),
            before: before.before,
            after: after.after,
            whole_read: after.whole_read,
            around: Some(json!({ "id": record.id, "in": true, "rank": rank, "row": row })),
            rows: before.rows.into_iter().chain(after.rows).collect(),
        })
    }

    /// How many rows a view shows before `place`.
    async fn rank(
        &self,
        table: &Collection,
        viewed: &Viewed,
        asked: &Asked,
        order: &Order,
        place: &Place,
        by: &Principal,
    ) -> Result<u64> {
        if table.store != Store::Local {
            let records = self.records(table).await?;
            let rows: Vec<(i64, &Map<String, Value>)> = records
                .iter()
                .filter(|record| viewed.find.as_ref().is_none_or(|find| find.finds(&record.data)))
                .map(|record| (record.number, &record.data))
                .collect();
            return Ok(found::rank_of(order, &viewed.held, &rows, place));
        }
        let (shape, way, gather) = self.planned(table, viewed, asked, by).await?;
        let place = place.clone();
        self.rows(&table.project)
            .db()
            .call(move |connection| rank_before(connection, &shape, way, gather, &place))
            .await
    }

    /// A window of a table kept outside the database: read whole, and
    /// windowed by the same rules.
    async fn window_in_hand(
        &self,
        table: &Collection,
        viewed: &Viewed,
        asked: &Asked,
        from: &From,
        limit: usize,
    ) -> Result<Answer> {
        let order = Order::of(&viewed.columns, asked.sort.as_ref())?;
        let records = self.records(table).await?;
        // What is searched for is no part of what the rules read in hand: the rows it does not find are left out first.
        let found: Vec<&Record> = records
            .iter()
            .filter(|record| viewed.find.as_ref().is_none_or(|find| find.finds(&record.data)))
            .collect();
        let rows: Vec<(i64, &Map<String, Value>)> =
            found.iter().map(|record| (record.number, &record.data)).collect();
        let window = in_hand(&order, &viewed.held, &rows, from, limit);
        let said = self.commented(table).await?;
        Ok(Answer {
            rows: window
                .rows
                .iter()
                .map(|at| {
                    let record = found[*at].clone();
                    let commented = said.get(&record.id).copied().unwrap_or(0);
                    handed(table, record, asked.fields.as_deref(), commented)
                })
                .collect(),
            after: window.after.map(|place| cursor(&viewed.mark, &place)),
            before: window.before.map(|place| cursor(&viewed.mark, &place)),
            looked: records.len() as u64,
            short: false,
            rank: window.rank,
            whole_read: (records.len() > WHOLE_READ).then_some(records.len()),
            around: None,
        })
    }

    /// Each of `ids` as the view `asked` of shows it: whether it shows it
    /// (`in`), where it stands in its order (`place`, as a cursor of it),
    /// and the row; one that is not a row of the table is said not to be.
    pub async fn window_ids(&self, table: &Collection, asked: &Asked, ids: &[String]) -> Result<Vec<Value>> {
        let viewed = viewed(table, asked)?;
        let order = Order::of(&viewed.columns, asked.sort.as_ref())?;
        let mut found = Vec::new();
        let said_on = self.commented(table).await?;
        for id in ids.iter().take(kernel::window::MOST) {
            let number = id.rsplit_once('-').and_then(|(_, number)| number.parse::<i64>().ok());
            let record = match number {
                Some(number) if table.owns(id) => self.one(table, id, number).await?,
                _ => None,
            };
            let Some(record) = record else {
                found.push(json!({ "id": id, "in": false, "row": null }));
                continue;
            };
            let shown = found::shown(&viewed.held, viewed.find.as_ref(), &record.data);
            let place = cursor(&viewed.mark, &order.place(&record.data, record.number));
            let said = said_on.get(id).copied().unwrap_or(0);
            let row = handed(table, record, asked.fields.as_deref(), said);
            found.push(json!({ "id": id, "in": shown, "place": place, "row": row }));
        }
        Ok(found)
    }

    /// How many rows the view `asked` of shows, how many the table has,
    /// how many are in each band of the column it is gathered by, and the
    /// figures asked (each a column and one of `kernel::columns::FIGURES`;
    /// the view's own where none are). `whole` is false where not every
    /// row was gone through yet, and each count is then at least so many.
    pub async fn counted_rows(
        &self,
        table: &Collection,
        asked: &Asked,
        figures: Option<BTreeMap<String, String>>,
    ) -> Result<Value> {
        let viewed = viewed(table, asked)?;
        let figures: Vec<(Column, String)> = figures
            .unwrap_or_else(|| viewed.figures.clone())
            .into_iter()
            .filter_map(|(name, which)| {
                let column = viewed.columns.iter().find(|column| column.name == name)?.clone();
                kernel::columns::fits(&column.kind, &which).then_some((column, which))
            })
            .collect();
        let group = viewed.group.clone().filter(|_| asked.band.is_none());
        let of = self.count(table).await? as u64;
        let figured: Vec<Column> = figures.iter().map(|(column, _)| column.clone()).collect();
        let off_index = match viewed.held.is_empty() && viewed.find.is_none() {
            true => self.off_index(table, of, group.as_ref(), &figures).await?,
            false => None,
        };
        let (count, counted_bands, said, whole, looked) = match off_index {
            Some((bands, said)) => (of, bands, said, true, of),
            None => {
                let (counting, whole) = self.counting(table, &viewed, group.as_ref(), &figured).await?;
                let said = figures
                    .iter()
                    .enumerate()
                    .map(|(at, (column, which))| {
                        counting.tallies.get(at).and_then(|tally| tally.figure(&column.kind, which))
                    })
                    .collect();
                (counting.count, counting.bands, said, whole, if whole { of } else { counting.looked })
            }
        };
        let bands = group.as_ref().map(|group| ordered_bands(group, &counted_bands));
        let said_figures: Map<String, Value> = figures
            .iter()
            .zip(said)
            .map(|((column, _), figure)| (column.name.clone(), json!(figure)))
            .collect();
        Ok(json!({
            "count": count,
            "of": of,
            "bands": bands.as_ref().map(|(bands, _)| bands.iter().map(|(key, count)| json!({ "key": key, "count": count })).collect::<Vec<_>>()),
            "more_bands": bands.as_ref().map_or(0, |(_, more)| *more),
            "figures": said_figures,
            "whole": whole,
            "looked": looked,
        }))
    }

    /// What is counted of a view held to nothing, read off the indexes of
    /// the columns asked about, where each part can be: the bands of a
    /// column that holds one value (not more of them than are shown), and
    /// the figures [`kernel::window::statements::figure`] says. Nothing
    /// where any part reads the rows.
    async fn off_index(
        &self,
        table: &Collection,
        of: u64,
        group: Option<&Column>,
        figures: &[(Column, String)],
    ) -> Result<Option<(BTreeMap<String, u64>, Vec<Option<f64>>)>> {
        if table.store != Store::Local {
            return Ok(None);
        }
        let indexed = self.indexed(table).await?;
        let on_index = |column: &Column| indexed.contains(&column.name);
        if group.is_some_and(|group| !on_index(group) || group.how() != Some(How::Value)) {
            return Ok(None);
        }
        let (project, name) = (table.project.as_str(), table.name.as_str());
        let mut asked = Vec::new();
        for (column, which) in figures {
            match statements::figure(project, name, column, which).filter(|_| on_index(column)) {
                Some(statement) => asked.push((statement, which.clone())),
                None => return Ok(None),
            }
        }
        let group = group.cloned();
        let (project, name) = (project.to_string(), name.to_string());
        self.rows(&project)
            .db()
            .call(move |connection| off_index(connection, &project, &name, group.as_ref(), &asked, of))
            .await
    }

    /// What is counted of a view, so far: from what was kept of it where
    /// nothing was written since, on from where that got to.
    async fn counting(
        &self,
        table: &Collection,
        viewed: &Viewed,
        group: Option<&Column>,
        figured: &[Column],
    ) -> Result<(Counting, bool)> {
        if table.store != Store::Local {
            let mut counting = Counting::default();
            for record in self.records(table).await? {
                if viewed.find.as_ref().is_none_or(|find| find.finds(&record.data)) {
                    counting.take(&viewed.held, group, figured, &record.data);
                }
            }
            return Ok((counting, true));
        }
        let asked = format!(
            "{}|{}|{:?}|{:?}|{:?}",
            table.project,
            table.name,
            viewed.find,
            viewed.held,
            (group.map(|group| &group.name), figured.iter().map(|column| &column.name).collect::<Vec<_>>())
        );
        let written = self.counted.mark(&table.project, &table.name);
        let kept = self.counts.kept(&asked, written);
        let (mut counting, mut from, whole) = kept.unwrap_or((Counting::default(), i64::MIN, false));
        if whole {
            return Ok((counting, true));
        }
        let mut wanted: Vec<String> = viewed.held.iter().map(|held| held.column.name.clone()).collect();
        wanted.extend(group.map(|group| group.name.clone()));
        wanted.extend(figured.iter().map(|column| column.name.clone()));
        wanted.extend(
            viewed.find.iter().flat_map(|find| find.columns.iter().map(|column| column.name.clone())),
        );
        wanted.sort();
        wanted.dedup();
        let mut through = false;
        for _ in 0..PIECES {
            let piece = self.piece(table, &wanted, from).await?;
            for (_, data) in &piece {
                if viewed.find.as_ref().is_none_or(|find| find.finds(data)) {
                    counting.take(&viewed.held, group, figured, data);
                }
            }
            counting.looked += piece.len() as u64;
            match piece.last() {
                Some((number, _)) if piece.len() as u64 == PIECE => from = *number,
                _ => {
                    through = true;
                    break;
                }
            }
        }
        if written.is_some() && written == self.counted.mark(&table.project, &table.name) {
            self.counts.keep(asked, written, (counting.clone(), from, through));
        }
        Ok((counting, through))
    }

    /// One piece of a table's rows after the one numbered `from`, by
    /// number, with what each holds of `wanted`.
    async fn piece(
        &self,
        table: &Collection,
        wanted: &[String],
        from: i64,
    ) -> Result<Vec<(i64, Map<String, Value>)>> {
        let (project, name, wanted) = (table.project.clone(), table.name.clone(), wanted.to_vec());
        self.rows(&table.project)
            .db()
            .call(move |connection| piece(connection, &project, &name, &wanted, from))
            .await
    }
}

impl Records {
    /// A window of a table, for whoever may read its records.
    pub async fn window(&self, project: &str, table: &str, asked: &Asked, by: &Principal) -> Result<Answer> {
        let table = self.readable(project, table, by).await?;
        self.collections.window(&table, asked, by).await
    }

    /// Each of `ids` as a view of a table shows it.
    pub async fn window_ids(
        &self,
        project: &str,
        table: &str,
        asked: &Asked,
        ids: &[String],
        by: &Principal,
    ) -> Result<Vec<Value>> {
        let table = self.readable(project, table, by).await?;
        self.collections.window_ids(&table, asked, ids).await
    }

    /// What is counted of a view of a table.
    pub async fn counted_rows(
        &self,
        project: &str,
        table: &str,
        asked: &Asked,
        figures: Option<BTreeMap<String, String>>,
        by: &Principal,
    ) -> Result<Value> {
        let table = self.readable(project, table, by).await?;
        self.collections.counted_rows(&table, asked, figures).await
    }

    async fn readable(&self, project: &str, table: &str, by: &Principal) -> Result<Collection> {
        if !by.can("board:read") {
            by.require("records:read")?;
        }
        let name = blueprint::expr::slug(table);
        let table = self.collections.get(project, &name).await?;
        // One kept from whoever asks is not there for them.
        if table.open_to(by).is_err() {
            return Err(crate::collections::missing(project, &name));
        }
        table.may_read(by)?;
        Ok(table)
    }
}
