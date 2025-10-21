#!/usr/bin/env node
import yargs from 'yargs'
import { hideBin } from 'yargs/helpers'
import process from 'node:process'
import pullCommand from './commands/pull.ts'
import pushCommand from './commands/push.ts'

yargs(hideBin(process.argv))
  .scriptName('gslm')
  .usage('$0 <command> [options]')
  .command(pullCommand)
  .command(pushCommand)
  .demandCommand(1, 'You need at least one command before moving on')
  .help('h')
  .alias('h', 'help')
  .version()
  .alias('v', 'version')
  .strict()
  .epilog(
    'For more information, visit: https://github.com/gn00678465/google-sheet-languages-model'
  )
  .parse()
