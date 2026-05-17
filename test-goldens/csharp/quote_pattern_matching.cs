var result = shape switch {
    Circle(c) => c.Radius * c.Radius * Math.PI,
    Rectangle(r) => r.Width * r.Height,
    _ => 0,
}
