/**
 * Server is an HTTP server.
 */
class Server {
    private string $host;
    private int $port;

    public function getHost(): string {
        return $this->host;
    }

    public function getPort(): int {
        return $this->port;
    }
}
