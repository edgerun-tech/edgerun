/**
 * Protocol error types and helpers.
 */

export class ProtocolError extends Error {
  constructor(
    message: string,
    public code?: string,
    public cause?: unknown,
  ) {
    super(message)
    this.name = "ProtocolError"
  }
}

export class TransportError extends ProtocolError {
  constructor(message: string, cause?: unknown) {
    super(message, "TRANSPORT_ERROR", cause)
    this.name = "TransportError"
  }
}

export class ValidationError extends ProtocolError {
  constructor(message: string, cause?: unknown) {
    super(message, "VALIDATION_ERROR", cause)
    this.name = "ValidationError"
  }
}

export class AuthorizationError extends ProtocolError {
  constructor(message: string, cause?: unknown) {
    super(message, "AUTHORIZATION_ERROR", cause)
    this.name = "AuthorizationError"
  }
}

export class NotFoundError extends ProtocolError {
  constructor(resource: string, cause?: unknown) {
    super(`${resource} not found`, "NOT_FOUND", cause)
    this.name = "NotFoundError"
  }
}

export function isProtocolError(err: unknown): err is ProtocolError {
  return err instanceof ProtocolError
}

export function getErrorCode(err: unknown): string | undefined {
  if (isProtocolError(err)) return err.code
  return undefined
}

export function getErrorMessage(err: unknown): string {
  if (err instanceof Error) return err.message
  return String(err)
}
