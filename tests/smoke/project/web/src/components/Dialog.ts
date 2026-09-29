export class Dialog {
  private readonly el: HTMLDialogElement;

  constructor(el: HTMLDialogElement) {
    this.el = el;
    this.close = this.close.bind(this);
    el.addEventListener("cancel", this.close);
  }

  open(): void {
    this.el.showModal();
  }

  close(): void {
    this.el.close();
  }
}

export function dialogOf(el: HTMLDialogElement, props: Props): Dialog {
  return props.modal ? new Dialog(el) : new Dialog(el);
}

type Props = { modal: boolean };

export function whenClosed(dialog: Dialog, done: () => void): void {
  onClose(dialog, () => {
    done();
  });
  onClose(dialog, () => {
    dialog.close();
  });
}
