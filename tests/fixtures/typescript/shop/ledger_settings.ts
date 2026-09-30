// The instance bound to a name first, then exported as the default (#341).
class LedgerSettings {
  public LEDGER_NAME = "Ledger";
}

const ledgerSettings = new LedgerSettings();
export default ledgerSettings;
