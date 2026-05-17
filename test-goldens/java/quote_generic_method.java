public < T > T fromJson(String json, Class<T> clazz) {
    return gson.fromJson(json, clazz);
}
