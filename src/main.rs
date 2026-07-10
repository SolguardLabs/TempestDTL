use tempest_dtl::runtime;

fn main() {
    if let Err(error) = runtime::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
