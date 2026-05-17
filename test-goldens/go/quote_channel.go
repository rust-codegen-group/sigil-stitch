ch := make(chan int, 10)
ch <-42
val := <-ch
