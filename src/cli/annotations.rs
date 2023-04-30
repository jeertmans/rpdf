use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};
use lopdf::{Document, Object, ObjectId};
use owo_colors::OwoColorize;
use tabled::{
    builder::Builder,
    settings::{style::BorderColor, Color, Panel, Style},
};
use termcolor::WriteColor;

use super::traits::Execute;

/// Stats command.
#[derive(Args, Clone, Debug)]
struct Stats {
    /// PDF filepath.
    file: PathBuf,
    /// Show per page statistics.
    #[clap(short, long)]
    per_page: bool,
}

impl Execute for Stats {
    fn execute<W>(&self, stdout: &mut W) -> Result<()>
    where
        W: WriteColor,
    {
        let document = Document::load(&self.file)
            .with_context(|| format!("Failed to read PDF from: {}", self.file.to_str().unwrap()))?;
        let mut counters = vec![];
        let mut subtypes = HashSet::new();

        for page in document.page_iter() {
            let mut counter = HashMap::new();
            for annotation in document.get_page_annotations(page) {
                let subtype = annotation
                    .get_deref(b"Subtype", &document)
                    .and_then(Object::as_name_str)
                    .unwrap_or("");

                *counter.entry(subtype).or_insert(0) += 1;
                subtypes.insert(subtype.to_owned());
            }
            counters.push(counter);
        }

        let mut builder = Builder::default();
        let mut subtypes: Vec<_> = subtypes.iter().collect();
        subtypes.sort();

        if subtypes.is_empty() {
            writeln!(stdout, "No annotation was found in the given file.")?;
            return Ok(());
        }

        if self.per_page {
            let mut header = Vec::with_capacity(1 + subtypes.len());
            header.push("Page no.".to_string());

            for subtype in &subtypes {
                header.push(subtype.to_string());
            }
            builder.set_header(header);
            for (i, counter) in counters.iter().enumerate() {
                if counter.is_empty() {
                    continue;
                }

                let mut record = Vec::with_capacity(1 + subtypes.len());
                record.push(format!("{}", i + 1));

                for subtype in &subtypes {
                    record.push(counter.get(subtype.as_str()).map_or_else(
                        || {
                            if stdout.supports_color() {
                                "0".dimmed().to_string()
                            } else {
                                "0".to_string()
                            }
                        },
                        |i| i.to_string(),
                    ));
                }

                builder.push_record(record);
            }
        } else {
            builder.set_header(subtypes.clone());

            let mut record = Vec::with_capacity(subtypes.len());
            let mut total = HashMap::with_capacity(subtypes.len());

            for counter in counters {
                for (subtype, count) in counter {
                    *total.entry(subtype).or_insert(0) += count;
                }
            }

            for subtype in &subtypes {
                record.push(total.get(subtype.as_str()).unwrap_or(&0).to_string());
            }

            builder.push_record(record);
        }

        let mut table = builder.build();
        table
            .with(Panel::header(format!(
                "Annotations stats for: {}",
                self.file.to_str().unwrap()
            )))
            .with(Style::modern());

        if stdout.supports_color() {
            table.with(BorderColor::filled(Color::FG_GREEN));
        }

        writeln!(stdout, "{table}")?;

        Ok(())
    }
}

/// Merge command.
#[derive(Args, Clone, Debug)]
struct Merge {
    /// PDF filepaths (at least two files).
    #[clap(num_args(2..), value_names = ["FILE 1", "FILE 2"], next_line_help = true, required = true)]
    files: Vec<PathBuf>,
    /// Output file where resulting PDF is written.
    #[clap(short, long, default_value = "merged_annotations.pdf")]
    dest: PathBuf,
    /// Exclude a given annotation type when merging (multiple values allowed).
    ///
    /// This is especially useful to avoid duplicating links, which are
    /// categorized as "annotations" too. Excluded annotation will only be
    /// kept in <FILE 1>.
    #[clap(short, long, default_value = "Link", action = ArgAction::Append)]
    exclude: Vec<String>,
}

/// Get mutable annotations (references) to a given page id.
fn get_page_annotations_mut(document: &mut Document, page_id: ObjectId) -> &mut Vec<Object> {
    let page = document.get_dictionary(page_id).unwrap();

    match page.get(b"Annots") {
        Ok(Object::Reference(ref id)) => {
            document
                .get_object_mut(*id)
                .and_then(Object::as_array_mut)
                .unwrap()
        },
        Ok(Object::Array(_)) => {
            let page = document.get_dictionary_mut(page_id).unwrap();
            page.get_mut(b"Annots")
                .and_then(Object::as_array_mut)
                .unwrap()
        },
        Err(_) => {
            let page = document.get_dictionary_mut(page_id).unwrap();
            page.set(b"Annots".to_owned(), vec![]);
            page.get_mut(b"Annots")
                .and_then(Object::as_array_mut)
                .unwrap()
        },
        _ => unreachable!(),
    }
}

impl Execute for Merge {
    fn execute<W>(&self, stdout: &mut W) -> Result<()>
    where
        W: WriteColor,
    {
        let mut main = Document::load(&self.files[0]).with_context(|| {
            format!(
                "Failed to read PDF from: {}",
                self.files[0].to_str().unwrap()
            )
        })?;

        let mut annotations_map = HashMap::new();

        for file in &self.files[1..] {
            let document = Document::load(file)
                .with_context(|| format!("Failed to read PDF from: {}", file.to_str().unwrap()))?;
            for page in document.page_iter() {
                document
                    .get_page_annotations(page)
                    .into_iter()
                    .filter(|annotation| {
                        let subtype = annotation
                            .get_deref(b"Subtype", &document)
                            .and_then(Object::as_name_str)
                            .unwrap_or("");

                        return !self.exclude.iter().any(|e| subtype == e);
                    })
                    .for_each(|annotation| {
                        let id = main.add_object(annotation.clone());
                        annotations_map
                            .entry(page)
                            .or_insert(vec![])
                            .push(Object::Reference(id));
                    });
            }
        }

        for (page, new_ann) in annotations_map.iter_mut() {
            let current_ann = get_page_annotations_mut(&mut main, *page);

            current_ann.append(new_ann);
        }

        main.save(&self.dest)?;

        writeln!(
            stdout,
            "Successfully merged annotations from {} files to {}",
            self.files.len(),
            self.dest.to_str().unwrap()
        )?;

        Ok(())
    }
}

/// Get annotation ids to a given page id.
fn get_page_annotations(document: &Document, page_id: ObjectId) -> Vec<ObjectId> {
    let page = document.get_dictionary(page_id).unwrap();
    let mut ids = vec![];

    match page.get(b"Annots") {
        Ok(Object::Reference(ref id)) => {
            document
                .get_object(*id)
                .and_then(Object::as_array)
                .unwrap()
                .iter()
                .flat_map(Object::as_reference)
                .for_each(|id| ids.push(id))
        },
        Ok(Object::Array(a)) => {
            a.iter()
                .flat_map(Object::as_reference)
                .for_each(|id| ids.push(id))
        },
        Err(_) => {},
        _ => unreachable!(),
    }
    ids
}

/// Strip command.
#[derive(Args, Clone, Debug)]
struct Strip {
    /// PDF filepath.
    file: PathBuf,
    /// Output file where resulting PDF is written.
    #[clap(short, long, default_value = "stripped_annotations.pdf")]
    dest: PathBuf,
    /// Exclude a given annotation type from stripping (multiple values
    /// allowed).
    #[clap(short, long, default_value = "Link", action = ArgAction::Append)]
    exclude: Vec<String>,
}

impl Execute for Strip {
    fn execute<W>(&self, stdout: &mut W) -> Result<()>
    where
        W: WriteColor,
    {
        let mut document = Document::load(&self.file)
            .with_context(|| format!("Failed to read PDF from: {}", self.file.to_str().unwrap()))?;

        let mut delete_ids = vec![];

        for page in document.page_iter() {
            for id in get_page_annotations(&document, page) {
                let subtype = document
                    .get_dictionary(id)
                    .unwrap()
                    .get_deref(b"Subtype", &document)
                    .and_then(Object::as_name_str)
                    .unwrap_or("");

                if !self.exclude.iter().any(|e| subtype == e) {
                    delete_ids.push(id);
                }
            }
        }

        for id in delete_ids {
            document.delete_object(id);
        }

        document.save(&self.dest)?;

        writeln!(
            stdout,
            "Successfully striped annotations from {} to {}",
            self.file.to_str().unwrap(),
            self.dest.to_str().unwrap()
        )?;

        Ok(())
    }
}

/// Annotations subcommand.
#[derive(Clone, Debug, Subcommand)]
enum AnnotationsSubcommand {
    /// Retrieves annotations statistics.
    Stats(Stats),
    /// Merge annotations from multiple files into one.
    Merge(Merge),
    /// Strip annotations from a given file.
    Strip(Strip),
}

/// Work with PDF annotations.
#[derive(Debug, Parser)]
#[clap(subcommand_required = true)]
pub struct AnnotationsCommand {
    /// Optional subcommand.
    #[command(subcommand)]
    subcommand: AnnotationsSubcommand,
}

impl Execute for AnnotationsCommand {
    fn execute<W>(&self, stdout: &mut W) -> Result<()>
    where
        W: WriteColor,
    {
        match &self.subcommand {
            AnnotationsSubcommand::Stats(stats) => stats.execute(stdout),
            AnnotationsSubcommand::Merge(merge) => merge.execute(stdout),
            AnnotationsSubcommand::Strip(strip) => strip.execute(stdout),
        }
    }
}
