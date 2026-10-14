;;; init.el --- a small Emacs setup -*- lexical-binding: t -*-
;;; Commentary:
;; Packages and keys.
;;; Code:

(require 'package)
(setq inhibit-startup-screen t
      fill-column 100)

(defvar my-ledger-file "~/ledger.dat"
  "Where the ledger lives.")

(defun my-open-ledger ()
  "Open the ledger file."
  (interactive)
  (find-file my-ledger-file))

(use-package magit
  :ensure t
  :bind (("C-x g" . magit-status))
  :config
  (setq magit-display-buffer-function #'magit-display-buffer-same-window-except-diff-v1))

(add-hook 'prog-mode-hook #'display-line-numbers-mode)
(global-set-key (kbd "C-c l") #'my-open-ledger)

(provide 'init)
;;; init.el ends here
