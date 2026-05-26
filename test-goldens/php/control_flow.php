function classify(int $x): string {
    if ($x > 0) {
        return "positive";
    } elseif ($x < 0) {
        return "negative";
    } else {
        return "zero";
    }
}
