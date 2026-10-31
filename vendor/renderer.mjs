import * as chartist from 'chartist';
import * as server from 'chartist/server';
import { parseHTML } from 'linkedom/worker';
globalThis.__vizChartist = chartist;
globalThis.__vizChartistServer = server;
globalThis.prepareViz = () => {
  const { document } = parseHTML('<!doctype html><html><body></body></html>');
  globalThis.document = document;
  const container = document.createElement('div');
  document.body.append(container);
  return container;
};
globalThis.runViz = (render, container) => {
  const result = render(container);
  if (result && typeof result.then === 'function') throw new Error('chart rendering must be synchronous');
  return container.innerHTML;
};
