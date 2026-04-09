;; Builtin imports
(require-builtin steel/strings)
(require-builtin steel/time)

;; Cogs imports
(require "helix/editor.scm")
(require "helix/misc.scm")
(require "helix/components.scm")
(require "helix/static.scm")

;; dylibs imports
(#%require-dylib "libhelix_discord_rpc"
  (only-in
    DiscordRPC::new
    DiscordRPC::set_activity
    DiscordRPC::set_idle))

(define server (DiscordRPC::new))
(define is-connected #false) ; a bit misleading, this var means whether or not we should send events
(define current-doc-id #false)

(define (refresh-presence)
  (when is-connected
    (let ([doc-path (and current-doc-id (editor-document->path current-doc-id))])
      (DiscordRPC::set_activity
        server
        (if doc-path (to-string doc-path) "<unnamed buffer>")
        (helix-find-workspace)
        (+ (get-current-line-number) 1) ; line and col are 0-indexed
        (+ (get-current-column-number) 1)))))

; We only probably only need selection-did-change and document-changed
(register-hook! 'selection-did-change
  (lambda (view-id)
    (refresh-presence)))

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
                               (begin
                                 (set! is-connected #true)
                                 "Websocket connected")))

(provide discord-rpc-connect)
