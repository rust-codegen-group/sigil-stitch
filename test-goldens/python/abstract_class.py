from abc import ABC, abstractmethod


class BaseController(ABC):
    @abstractmethod
    def handle_request(self, req: Request) -> Response: ...

    def log(self) -> None:
        print('handled')
