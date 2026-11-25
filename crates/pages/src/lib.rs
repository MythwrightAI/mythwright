//! Pages, and the files they are made of.
//!
//! A document is a Markdown file and a mockup a folder that holds a page;
//! a library is written out as pages that anything can serve. What is here
//! is how each is read and written as files, and how a page is made of a
//! document (`made`): functions of a file's text, a folder and plain
//! values, which know no table and no record. It came out of `studio`,
//! which makes records of documents and mockups and asks who may write a
//! library out, and which offers what is here under the names it had
//! (`studio::pages`, `studio::site`, `studio::documents`,
//! `studio::mockups`).

pub mod document;
pub mod made;
pub mod mockup;
pub mod site;
