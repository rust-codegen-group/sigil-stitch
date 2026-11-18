@app.route("/users")
def get_users():
    return jsonify(users)
