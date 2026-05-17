try (BufferedReader reader = new BufferedReader(new FileReader(path))) {
    String line = reader.readLine();
    System.out.println(line);
}
