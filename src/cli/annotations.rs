use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::{ArgAction, Args, Parser, Subcommand};
use log::{Level::Info, debug, error, info, log_enabled, trace, warn};
use lopdf::{Document, Object, ObjectId};
use owo_colors::OwoColorize;
use tabled::{
    builder::Builder,
    settings::{Color, Panel, Style, style::BorderColor},
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

        let document = Document::load(self.file)  
        
        .with_context(|| format!("Failed to read PDF from: {:?}.", self.file))?;  

        let mut counters = vec![];
        let mut subtypes = HashSet::new();

        for page in document.page_iter() {
            let mut counter = HashMap::new();
            for annotation in document
                .get_page_annotations(page)
                .with_context(|| format!("Failed to get page annotations for page ID {page:?}."))?
            {
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

            debug!("Summing counts from each page...");
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
            trace!("Stdout supports color so table will be colored");
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
    /// Overwrite output file if exists.
    #[clap(short = 'f', long = "force")]
    overwrite: bool,
}

/// Get mutable annotations (references) to a given page id.
fn get_page_annotations_mut(document: &mut Document, page_id: ObjectId) -> &mut Vec<Object> {
    match document.get_dictionary(page_id).unwrap().get(b"Annots") {
        Ok(Object::Reference(ref id)) => {
            trace!("This page contains a reference to a vector of annotations");
            document
                .get_object_mut(*id)
                .and_then(Object::as_array_mut)
                .unwrap()
        },
        Ok(Object::Array(_)) => {
            trace!("This page contains a vector of annotations");
            let page = document.get_dictionary_mut(page_id).unwrap();
            page.get_mut(b"Annots")
                .and_then(Object::as_array_mut)
                .unwrap()
        },
        Err(_) => {
            trace!("This page (ID: {:?}) does not contain any annotations, inserting an empty array.", page_id);
            let page_map = document
                .get_dictionary_mut(page_id)
                .unwrap()
                .as_hashmap_mut();
            Object::as_array_mut(
                page_map
                    .entry(b"Annots".to_vec())
                    .or_insert(Object::Array(vec![])),
            )
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
        if self.dest.exists()
            && !self.overwrite
            && !dialoguer::Confirm::new()
                .with_prompt(format!(
                    "Output file {:?} already exists. Do you want to overwrite it?",
                    self.dest
                ))
                .interact()
                .unwrap_or(false)
        {
            return Ok(());
        }
        if log_enabled!(Info) {
            let msg = format!(
                "Processing documents: {}",
                self.files
                    .iter()
                    .enumerate()
                    .map(|(document_number, file)| format!("{:?} (#{})", file, document_number))
                    .collect::<Vec<String>>() // Collect into a Vec<String>
                    .join(", ")
            );

            info!("{}.", msg);
        }
        let mut main = Document::load(&self.files[0]).with_context(|| {
            format!(
                "Failed to read PDF from: {}",
                self.files[0].to_str().unwrap()
            )
        })?;

        let pages = main.get_pages();
        debug!("Reference document contains {} pages", pages.len());

        // Maps page number (note object id) to annotations
        let mut annotations_map = HashMap::new();

        for (document_number, file) in (1..).zip(&self.files[1..]) {
            debug!("Processing document #{document_number}");
            let document = Document::load(file)
                .with_context(|| format!("Failed to read PDF from: {}", file.to_str().unwrap()))?;

            for (page_number, page) in (1u32..).zip(document.page_iter()) {
                if !pages.contains_key(&page_number) {
                    warn!(
                        "Reference document does not contain page number {}. Annotations from \
                         this page will be ignored.",
                        page_number
                    );
                }
                document
                    .get_page_annotations(page)
                    .with_context(|| {
                        format!("Failed to get page annotations for page ID {page:?}.")
                    })?
                    .into_iter()
                    .filter(|annotation| {
                        let subtype = annotation
                            .get_deref(b"Subtype", &document)
                            .and_then(Object::as_name_str)
                            .unwrap_or("");

                        return !self.exclude.iter().any(|e| subtype == e);
                    })
                    .for_each(|annotation| {
                        trace!(
                            "Found annotation on page {page_number} in document \
                             #{document_number}, inserting it inside reference document"
                        );
                        let id = main.add_object(annotation.clone());
                        annotations_map
                            .entry(page_number)
                            .or_insert(vec![])
                            .push(Object::Reference(id));
                    });
            }
        }

        info!("Updating the annotation arrays in reference document");
        for (page_number, new_ann) in annotations_map.iter_mut() {
            match pages.get(page_number) {
                Some(page_id) => {
                    debug!(
                        "Retrieving a mutable reference to page's {page_number} annotations in \
                         reference document"
                    );
                    let current_ann = get_page_annotations_mut(&mut main, *page_id);

                    current_ann.append(new_ann);
                },
                None => error!("Main document does not have page number {page_number}"),
            }
        }

        main.save(&self.dest)?;

        writeln!(
            stdout,
            "Successfully merged annotations from {} files to {:?}.",
            self.files.len(),
            self.dest.to_str().unwrap()
        )?;

        Ok(())
    }
}

/// Get annotation ids of a given page id.
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
