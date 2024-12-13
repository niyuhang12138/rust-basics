use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let g: Vec<_> = glob::glob("*.txt")?.collect();
    Ok(())
}
