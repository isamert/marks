use itertools::Itertools;
use rayon::prelude::*;
use structopt::StructOpt;
use std::io;

use marks::args::Args;
use marks::marks::Marks; // TODO: what

fn main() -> Result<(), io::Error> {
    let args = Args::from_args();
    let app = Marks::new(&args);

    if args.debug {
        println!("{:#?}", app.args);
    }

    let mut results = app
        .find_files()
        .collect::<Vec<_>>()
        .par_iter()
        .filter_map(|f| app.search_file(&f))
        .flatten()
        .collect::<Vec<_>>();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    let mut iter: Box<dyn Iterator<Item = _>> = Box::new(results.iter_mut());

    if args.only_headers {
        iter = Box::new(
            iter
                .unique_by(|x| format!("{}:{}", x.file_path, x.headers.last().map_or(0, |x| x.line)))
                .map(|x| { x.is_header = true; x })
        );
    }

    if let Some(count) = args.count {
        iter = Box::new(
            iter.take(count)
        );
    }

    iter.for_each(|x| x.print());

    Ok(())
}
