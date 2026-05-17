var results = items.Where(x => x.IsActive).Select(x => x.Name).ToList();
