use rand::Rng;

fn main() {
    let mut rng = rand::rng();
    let n: u32 = rng.random_range(0..10);
    println!("Random number: {n}");
    let m: u32 = n + 1;
    println!("Random number: {m}");
}
