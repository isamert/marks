use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "marks")]
/// A search-engine like search tool for markdown and org-mode files.
pub struct Args {
    /// Activate debug mode
    #[structopt(short, long)]
    pub debug: bool,

    // TODO: newlines for the help string?
    /// The query.
    ///
    /// An example query may look like this:
    ///    '"this" is a `(test|trial)` query -badword'
    /// This query requires
    ///   - the word "this" to be either in the title hierarchy or in the line.
    ///   - regex "(test|trial)" to either in the title hierarchy or in the line.
    ///   - "badword" to be not in the title hierarchy or the line itself.
    /// Rest of the characters are matched in fuzzy fashion.
    #[structopt(short, long)]
    pub query: String,

    /// Where to search for.
    #[structopt(short, long)]
    pub path: String,

    /// List of extensions for org files.
    #[structopt(short, long, default_value="org")]
    pub org_extension: Vec<String>,

    /// List of extensions for org files.
    #[structopt(short, long, default_value="md")]
    pub md_extension: Vec<String>,

    /// How many results do you want?
    #[structopt(short, long)]
    pub count: Option<usize>,

    /// Don't search for org files.
    #[structopt(long)]
    pub no_org: bool,

    /// Don't search for markdown files.
    #[structopt(long)]
    pub no_markdown: bool,

    /// Whether to search in files too.
    #[structopt(long)]
    pub search_filename: bool,

    /// Don't use colors for the output.
    #[structopt(long)]
    pub no_color: bool,

    /// Don't include headers to output.
    #[structopt(long)]
    pub no_headers: bool,

    /// A seperator to insert between headers while outputting.
    #[structopt(long, default_value="/")]
    pub header_seperator: String,

    // Output file, stdout if not present
    // #[structopt(parse(from_os_str))]
    // output: Option<PathBuf>,
}
