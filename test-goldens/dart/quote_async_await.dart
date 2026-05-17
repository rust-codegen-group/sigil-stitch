Future<String> fetchData(String url) async {
  final response = await http.get(Uri.parse(url));
  return response.body;
}
