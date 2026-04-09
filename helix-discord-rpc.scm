;; Builtin imports
(require-builtin steel/strings)
(require-builtin steel/time)

;; Cogs imports
(require "helix/editor.scm")
(require "helix/misc.scm")
(require "helix/components.scm")

;; dylibs imports
(#%require-dylib "libhelix_discord_rpc"
  (only-in
    DiscordRPC::new
    DiscordRPC::connect
    DiscordRPC::set_activity
    DiscordRPC::set_idle))

(define server (DiscordRPC::new))
(define is-connected #false)
(define row 0)
(define col 0)
(define current-doc-id #false)

(define (get-cursor-row-col)
  (match (current-cursor)
    [#f ; checks whether if cursor is invisible?
      (set-status! "No primary cursor is visible")]
    [(list pos kind) ; when visible, it's a list of Position? & CursorKind (wtf is that? [normal/select/visual?])
      (set! row (position-row pos))
      (set! col (position-col pos))]))

(define (refresh-presence)
  (when is-connected
    (get-cursor-row-col)
    (let ([doc-path (and current-doc-id (editor-document->path current-doc-id))])
      (DiscordRPC::set_activity
        server
        (if doc-path (to-string doc-path) "<unnamed buffer>")
        (helix-find-workspace)
        row
        col))))

; We only probably only need selection-did-change and document-changed
(register-hook! 'selection-did-change
  (lambda (view-id) (refresh-presence)))

; FIXME: maybe this will change to document-did-change
(register-hook! 'document-changed
  (lambda (doc-id old_text)
    (set! current-doc-id doc-id)
    (refresh-presence)))

; FIXME: maybe this will change to document-did-open
(register-hook! 'document-opened
  (lambda (doc-id)
    (set! current-doc-id doc-id)
    (refresh-presence)))

(register-hook! 'document-focus-lost
  (lambda (doc-id)
    (set! current-doc-id doc-id)
    (refresh-presence)))

;;@docs
; Connects the server to discord's websocket
(define (discord-rpc-connect) (
                               if
                               is-connected
                               "Websocket already connected"
                               (begin (DiscordRPC::connect server)
                                 (set! is-connected #true)
                                 (refresh-presence)
                                 "Websocket connected")))

(provide discord-rpc-connect)
