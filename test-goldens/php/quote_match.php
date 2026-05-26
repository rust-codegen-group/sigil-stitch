$result = match ($code) {
    200 => "OK",
    404 => "Not Found",
    default => "Unknown",
};
