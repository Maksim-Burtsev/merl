from repos import Notifier


class BaseJob:
    def run(self) -> None:
        pass


class ImportJob(BaseJob):
    def run(self) -> None:
        pass


class ExportJob(BaseJob):
    def run(self) -> None:
        pass


class QuietJob(BaseJob):
    pass


class NightlyJob(ImportJob):
    def run(self) -> None:
        pass


class LoudNotifier(Notifier):
    def send(self, text: str) -> None:
        pass


class WebhookNotifier:
    def send(self, text: str) -> None:
        pass


class Batch:
    def send(self, text: str, retries: int) -> None:
        pass


class Sweeper:
    def sweep(self) -> None:
        pass


class NightlySweeper(Sweeper):
    def sweep(self) -> None:
        pass
