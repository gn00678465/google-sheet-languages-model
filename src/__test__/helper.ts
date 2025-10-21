import { expect } from 'vitest'
import {
  FlatLanguagesContent,
  LanguagesModel,
  NestLanguagesContent,
} from '../core/LanguagesModel.ts'
import { dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

export const __dirname = dirname(fileURLToPath(import.meta.url))

export const languages = ['en', 'fr', 'es'] as const

export const flatLanguagesContentExample: FlatLanguagesContent = {
  en: {
    name: 'name',
    job: 'job',
    'nest.object.example1': 'example1',
    'nest.object.example2': 'example2',
  },
  fr: { name: 'nom', job: 'emploi' },
  es: { name: 'nombre', job: 'trabajo' },
}

export const nestLanguagesContentExample: NestLanguagesContent = {
  en: {
    name: 'name',
    job: 'job',
    nest: { object: { example1: 'example1', example2: 'example2' } },
  },
  fr: { name: 'nom', job: 'emploi' },
  es: { name: 'nombre', job: 'trabajo' },
}

export const expectLanguagesModel = (languagesModel: LanguagesModel) => {
  expect(languagesModel.getFlat()).toStrictEqual(flatLanguagesContentExample)
  expect(languagesModel.getNest()).toStrictEqual(nestLanguagesContentExample)
}
