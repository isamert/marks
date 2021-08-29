use rayon::prelude::*;
use std::io;

use marks::args::Args;
use marks::marks::Marks; // TODO: what

#[paw::main]
fn main(args: Args) -> Result<(), io::Error> {
    let count = args.count;
    let debug = args.debug;
    let app = Marks::new(&args);

    if debug {
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
    results
        .iter()
        .take(count.unwrap_or(usize::MAX))
        .for_each(|result| result.print(&args));

    Ok(())
}
