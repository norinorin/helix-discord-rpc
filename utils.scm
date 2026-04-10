(require-builtin steel/strings)

(require "helix/editor.scm")
(require "helix/components.scm")
(require "helix/static.scm")

(define (discord-rpc-normalize-path path)
  (if path
    (string-replace path "\\" "/")
    ""))

(define (discord-rpc-current-file-path)
  (let* ([view (editor-focus)]
         [doc-id (and view (editor->doc-id view))]
         [path (and doc-id (editor-document->path doc-id))])
    (if path path "")))

(define (discord-rpc-current-workspace-path)
  (let ([ws (helix-find-workspace)])
    (if ws ws "")))

(define (discord-rpc-cursor-line)
  (+ (get-current-line-number) 1))

(define (discord-rpc-cursor-column)
  (+ (get-current-column-number) 1))

(define (discord-rpc-basename path)
  (if path
    (let* ([normalized (discord-rpc-normalize-path path)]
           [parts (split-many normalized "/")])
      (if (null? parts)
        "Unknown"
        (car (reverse parts))))
    "Unknown"))

(define (discord-rpc-current-language)
  (let* ([view (editor-focus)]
         [doc-id (and view (editor->doc-id view))])
    (if doc-id
      (or (editor-document->language doc-id) "")
      "")))

(provide
  discord-rpc-current-file-path
  discord-rpc-current-workspace-path
  discord-rpc-current-language
  discord-rpc-cursor-line
  discord-rpc-cursor-column
  discord-rpc-basename)
