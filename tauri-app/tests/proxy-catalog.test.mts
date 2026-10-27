import test from 'node:test';
import assert from 'node:assert/strict';
import { catalogNodes, proxyCandidates, uniqueProxy } from '../frontend/utils/proxyCatalog.ts';

const response = {
  proxies: { top: {name:'top', type:'Vless', alive:true}, PROXY:{name:'PROXY',type:'Selector',all:['top','remote']}, DIRECT:{name:'DIRECT',type:'Direct'} },
  providers: {
    default: {vehicleType:'Compatible',proxies:[{name:'top',type:'Vless'}]},
    source: {vehicleType:'HTTP',proxies:[{name:'remote',type:'Trojan',alive:true}]},
  },
};

test('provider node details remain available without changing top-level nodes', () => {
  assert.equal(uniqueProxy(response,'top'), response.proxies.top);
  assert.equal(uniqueProxy(response,'remote'), response.providers.source.proxies[0]);
  assert.deepEqual(proxyCandidates(response,'remote').map(node => node.provider), ['source']);
  assert.equal(catalogNodes(response).length, 2);
  assert.equal(uniqueProxy({proxies:response.proxies},'top'), response.proxies.top);
});

test('equal names retain every provider identity instead of guessing the active source', () => {
  const duplicates = {...response, providers:{...response.providers, second:{vehicleType:'Inline',proxies:[{name:'remote',type:'Vless'}]}}};
  assert.equal(proxyCandidates(duplicates,'remote').length, 2);
  assert.equal(uniqueProxy(duplicates,'remote'), undefined);
  assert.equal(catalogNodes(duplicates).length, 3);
});


test('top-level/provider collisions are explicit and older flattened entries are not duplicated', () => {
  const collision = {...response, providers:{...response.providers, second:{vehicleType:'HTTP',proxies:[{name:'top',type:'Trojan'}]}}};
  assert.equal(proxyCandidates(collision,'top').length, 2);
  assert.equal(uniqueProxy(collision,'top'), undefined);
  const legacy = {...response, proxies:{...response.proxies, remote:{name:'remote',type:'Trojan','provider-name':'source'}}};
  assert.equal(proxyCandidates(legacy,'remote').length, 1);
  assert.equal(catalogNodes(legacy).length, 2);
});
