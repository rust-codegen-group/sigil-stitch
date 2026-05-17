fn process < T: Clone + Send + 'static >(item: T) -> T {
    item.clone()
}
