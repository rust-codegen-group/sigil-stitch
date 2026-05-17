List<String> names = users.stream()
.filter(u -> u.isActive())
.map(u -> u.getName())
.collect(Collectors.toList());
