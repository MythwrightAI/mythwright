// The parts of a told line that the daemon reads for whoever reads it
// (`told::read`), on the line's own page: rows of a project's table, a
// table whose cells are drawn as the table draws them (a choice as its
// tag, a day as a day, seconds as a length of time) and whose rows each
// open their form, with the whole of what they hold, in the drawer beside
// the page; runs, each with what it is a run of, how it went, when and how
// long, opening its run's page as a tab here; and measurements, each its
// latest value beside the median of its last few, written as the
// measurements page writes what each is of. A part the reader may not
// read says why in its place and shows nothing of it. What is said is
// `toldread.ts`, with its cases.

import { api } from '../api.ts';
import { collections, openTab, refreshCollections } from '../store.ts';
import { heading } from '../table.ts';
import { classes, when } from '../util.ts';
import { labelsSaid, measureSaid, noRows, rowsHead, runAddress, runsHead, secondsSaid, tookSaid, wentSaid } from '../toldread.ts';
import type { ListedRow, MeasuresPart, RowsColumn, RowsPart, RunsPart } from '../toldread.ts';
import { openRow } from './records.tsx';
import { Shown } from './table.tsx';

/** Opens a row in its form in the drawer beside the page: read whole, its table's fields read first where the page has not read them. */
async function openListed(project: string, table: string, id: string): Promise<void> {
  const row = await api.record(id);
  if (!(collections.value[project] ?? []).some((one) => one.name === table)) await refreshCollections(project);
  openRow(project, table, row);
}

/** One cell as its table draws it, or a number of seconds as a length of time. */
function Cell({ column, value }: { column: RowsColumn; value: unknown }) {
  if (column.seconds) return <>{secondsSaid(value)}</>;
  return <Shown field={column} value={value} people={[]} />;
}

export function RowsView({ part }: { part: RowsPart }) {
  const head = rowsHead(part);
  const columns = part.columns ?? [];
  const rows = part.rows ?? [];
  return (
    <section class="told-part told-rows" data-kind="rows">
      <h2 class="inbox-page-label">{head.title}</h2>
      {part.refused !== undefined ? (
        <p class="told-refused faint">{part.refused}</p>
      ) : rows.length === 0 ? (
        <p class="faint">{noRows(part)}</p>
      ) : (
        <div class="told-rows-scroll">
          <table class="told-rows-table">
            <thead>
              <tr>
                {columns.map((column) => (
                  <th key={column.name} scope="col" class={classes(column.seconds && 'num')}>
                    {heading(column)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <Row key={row.id} project={part.project} table={part.table} columns={columns} row={row} />
              ))}
            </tbody>
          </table>
        </div>
      )}
      {head.note && <p class="told-goes faint">{head.note}</p>}
    </section>
  );
}

function Row({ project, table, columns, row }: { project: string; table: string; columns: RowsColumn[]; row: ListedRow }) {
  if (row.gone) {
    return (
      <tr class="told-row gone" data-row={row.id}>
        <td colSpan={Math.max(columns.length, 1)} class="faint">
          {row.id} is no longer in {table}
        </td>
      </tr>
    );
  }
  const [first, ...rest] = columns;
  const said = first ? row.cells?.[first.name] : null;
  return (
    <tr class="told-row" data-row={row.id}>
      <td class="told-row-first">
        {/* The row pressed opens its form, with the whole of what it holds. */}
        <button type="button" class="link told-row-open" title={`Open ${row.id}`} onClick={() => void openListed(project, table, row.id)}>
          {first && said !== null && said !== undefined && said !== '' ? <Cell column={first} value={said} /> : row.id}
        </button>
      </td>
      {rest.map((column) => (
        <td key={column.name} class={classes(column.seconds && 'num')} data-label={heading(column)}>
          <Cell column={column} value={row.cells?.[column.name]} />
        </td>
      ))}
    </tr>
  );
}

export function RunsView({ part }: { part: RunsPart }) {
  const runs = part.runs ?? [];
  return (
    <section class="told-part told-runs" data-kind="runs">
      <h2 class="inbox-page-label">{runsHead(part)}</h2>
      {part.refused !== undefined ? (
        <p class="told-refused faint">{part.refused}</p>
      ) : runs.length === 0 ? (
        <p class="faint">Nothing has run yet.</p>
      ) : (
        <ol class="told-runs-list">
          {runs.map((run) => (
            <li key={run.id} class={classes('told-run', run.status)} data-run={run.id}>
              <a
                class="told-run-open"
                href={runAddress(run.id)}
                onClick={(event) => {
                  event.preventDefault();
                  openTab({ kind: 'run', id: run.id });
                }}
              >
                {run.called}
              </a>
              <span class="told-run-went">{wentSaid(run.status)}</span>
              <time class="faint" dateTime={new Date(run.started).toISOString()}>
                {when(run.started)}
              </time>
              <span class="faint told-run-took">{tookSaid(run)}</span>
            </li>
          ))}
        </ol>
      )}
    </section>
  );
}

export function MeasuresView({ part }: { part: MeasuresPart }) {
  const labels = labelsSaid(part.labels);
  return (
    <section class="told-part told-measures" data-kind="measures">
      <h2 class="inbox-page-label">Measured</h2>
      {part.refused !== undefined ? (
        <p class="told-refused faint">{part.refused}</p>
      ) : (
        <>
          {labels && <p class="told-goes faint">{labels}</p>}
          <dl class="told-measures-list">
            {(part.measures ?? []).map((measure) => {
              const said = measureSaid(measure);
              return (
                <div key={measure.name} class="told-measure" data-measure={measure.name}>
                  <dt>{measure.name}</dt>
                  {said ? (
                    <dd>
                      <strong>{said.latest}</strong> <span class="faint">latest; {said.median}</span>
                    </dd>
                  ) : (
                    <dd class="faint">nothing measured</dd>
                  )}
                </div>
              );
            })}
          </dl>
        </>
      )}
    </section>
  );
}
