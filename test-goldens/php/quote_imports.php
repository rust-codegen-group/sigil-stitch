use Psr\Log\LoggerInterface;
use Symfony\Component\HttpFoundation\Request;

$logger = new LoggerInterface();
$req = Request::createFromGlobals();
