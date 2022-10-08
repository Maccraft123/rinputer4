mod source;

fn main() {
    let devices = source::enumerate();
    println!("{:#?}", devices);
}
