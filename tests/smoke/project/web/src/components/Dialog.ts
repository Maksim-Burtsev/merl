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
