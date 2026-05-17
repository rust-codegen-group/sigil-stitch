def execute(query: String)(implicit ctx: ExecutionContext): Future[Result] = {
  Future(runQuery(query))
}
