;; Builtin imports
(require-builtin steel/strings)
(require-builtin steel/time)

(require "utils.scm")

(#%require-dylib "libhelix_discord_rpc"
  (only-in
    DiscordRPC::new
    DiscordRPC::set_activity
    DiscordRPC::set_idle))

; CONFIGURATION
; TODO: implement idle and dynamic asset base url
(define idle_timeout 0) ; 0 = disabled
(define base_asset_url "https://raw.githubusercontent.com/norinorin/helix-discord-rpc/refs/heads/main/assets/icons/") ; expects icons to be in png
(define (default-state-fn)
  (let* ([path (discord-rpc-current-file-path)]
         [filename (discord-rpc-basename path)]
         [row (discord-rpc-cursor-line)]
         [col (discord-rpc-cursor-column)])
    (string-append
      "Editing "
      filename
      " "
      (number->string row)
      ":"
      (number->string col))))
(define (default-details-fn)
  (let* ([workspace (discord-rpc-current-workspace-path)]
         [folder (discord-rpc-basename workspace)])
    (string-append "Workspace " folder)))
(define state-fn default-state-fn)
(define details-fn default-details-fn)

;;@docs
; Sets the idle timeout
(define (discord-rpc-set-idle-timeout timeout)
  (set! idle_timeout timeout)
  "Not implemented!")

(provide discord-rpc-set-idle-timeout)

;;@docs
; Sets the base asset url
(define (discord-rpc-set-base-asset-url url)
  (set! base_asset_url url)
  "Not implemented!")

(provide discord-rpc-set-base-asset-url)

;;@docs
; Registers a function that returns state string
(define (discord-rpc-register-state-fn function)
  (set! state-fn function))

(provide discord-rpc-register-state-fn)

;;@docs
; Registers a function that returns details string
(define (discord-rpc-register-details-fn function)
  (set! details-fn function))

(provide discord-rpc-register-details-fn)

; INTERNALS
(define server (DiscordRPC::new))
(define is-connected #false) ; a bit misleading, this var means whether or not we should send events

(define (refresh-presence)
  (when is-connected
    (DiscordRPC::set_activity
      server
      (discord-rpc-current-file-path)
      (details-fn)
      (state-fn))))

; We probably only need selection-did-change and document-changed
; Maybe throttle this function?
(register-hook! 'selection-did-change
  (lambda (view-id)
    (refresh-presence)))

; FIXME: maybe this will change to document-did-change
(register-hook! 'document-changed
  (lambda (doc-id old_text)
    (refresh-presence)))

; FIXME: maybe this will change to document-did-open
(register-hook! 'document-opened
  (lambda (doc-id)
    (refresh-presence)))

;;@docs
; Connects the server to discord's websocket
(define (discord-rpc-connect)
  (if
    is-connected
    "Websocket already connected"
    (begin
      (set! is-connected #true)
      "Websocket connected")))

(provide discord-rpc-connect)
