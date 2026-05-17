match value {
    Some(x) if x > 0 => println!("positive: {}", x),
    Some(0) => println!("zero"),
    None => println!("nothing"),
    _ => unreachable!(),
}
