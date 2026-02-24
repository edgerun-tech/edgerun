globalThis.process ??= {}; globalThis.process.env ??= {};
import './chunks/astro-designed-error-pages_pJXwdU1O.mjs';
import './chunks/astro/server_B5lkohcy.mjs';
import { s as sequence } from './chunks/index_DxG_ot85.mjs';

const onRequest$1 = (context, next) => {
  if (context.isPrerendered) {
    context.locals.runtime ??= {
      env: process.env
    };
  }
  return next();
};

const onRequest = sequence(
	onRequest$1,
	
	
);

export { onRequest };
