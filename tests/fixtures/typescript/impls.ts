import { Notifier } from "./repos";

export class BaseJob {
  run(): void {}
}

export class ImportJob extends BaseJob {
  run(): void {}
}

export class ExportJob extends BaseJob {
  run(): void {}
}

export class QuietJob extends BaseJob {}

export class NightlyJob extends ImportJob {
  run(): void {}
}

export class LoudNotifier implements Notifier {
  send(text: string): void {
    console.log(text);
  }
}

export class Sweeper {
  sweep(): void {}
}

export class NightlySweeper extends Sweeper {
  sweep(): void {}
}
