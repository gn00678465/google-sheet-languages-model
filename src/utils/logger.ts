/**
 * Simple CLI logger utilities
 * Provides colored output for different message types
 */

export const logger = {
  success(message: string) {
    console.log(`✓ ${message}`)
  },

  error(message: string) {
    console.error(`✗ Error: ${message}`)
  },

  warning(message: string) {
    console.warn(`⚠ Warning: ${message}`)
  },

  info(message: string) {
    console.log(`ℹ ${message}`)
  },

  log(message: string) {
    console.log(message)
  },

  /**
   * Print a divider line
   */
  divider() {
    console.log('─'.repeat(50))
  },

  /**
   * Print command start message
   */
  startCommand(command: string) {
    this.divider()
    this.info(`Starting ${command} command...`)
    this.divider()
  },

  /**
   * Print command completion message
   */
  completeCommand(command: string, duration?: number) {
    this.divider()
    const timeInfo = duration ? ` (${duration}ms)` : ''
    this.success(`${command} completed successfully${timeInfo}`)
    this.divider()
  },
}
