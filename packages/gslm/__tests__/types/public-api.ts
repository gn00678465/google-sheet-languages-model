import {
  SheetsClient,
  type CredentialsOptions,
  type Model,
  type SheetsClientOptions,
} from '../../index'

const credentials: CredentialsOptions = { accessToken: 'test-token' }
const options: SheetsClientOptions = { credentials }
const model: Model = {
  locales: ['en'],
  catalogs: { en: { greeting: 'Hello' } },
}

void SheetsClient.create(options)
void model
