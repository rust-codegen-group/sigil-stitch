val result = for {
  x <-fetchX()
  y <-fetchY(x)
}
yield (x, y)
