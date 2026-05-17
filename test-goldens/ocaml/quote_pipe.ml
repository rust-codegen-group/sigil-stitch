let result = input
|> List.map f
|> List.filter g
|> List.fold_left(+) 0
