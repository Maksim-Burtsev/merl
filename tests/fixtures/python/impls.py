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


class WrappedJob(
    BaseJob,  # a header black wrapped over several lines
    metaclass=type,
):
    def run(self) -> None:
        self.mop()

    def mop(self) -> None:
        pass


class Roster:
    jobs = sorted(
        BaseJob,
    )

    def run(self) -> None:
        pass

    def mop(self) -> None:
        pass


def wrapped(job: WrappedJob) -> None:
    job.mop()


class DeepJob(WrappedJob):
    def run(self) -> None:
        pass


class WideBase(
    object,
):
    def tick(self) -> None:
        pass


class Narrow(
    Sweeper, WideBase):
    def tick(self) -> None:
        self.sweep()
