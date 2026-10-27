import type { ProxiesResponse, ProxyGroup, ProxyNode } from '../types';

export interface ProxyCandidate {
  provider?: string;
  node: ProxyNode | ProxyGroup;
}

const groupTypes = ['Selector', 'URLTest', 'Fallback', 'LoadBalance', 'Relay'];
const builtins = ['Direct', 'Reject', 'RejectDrop', 'Compatible', 'Pass', 'PassRule'];

/** Keep provider identity instead of overwriting equal display names. */
export function proxyCandidates(response: ProxiesResponse, name: string): ProxyCandidate[] {
  const result: ProxyCandidate[] = [];
  const direct = response.proxies[name];
  if (direct && !(direct as ProxyNode)['provider-name']) result.push({ node: direct });
  for (const [provider, data] of Object.entries(response.providers || {})) {
    if (data.vehicleType === 'Compatible') continue;
    for (const node of data.proxies) {
      if (node.name === name) result.push({ provider, node });
    }
  }
  return result;
}

export function uniqueProxy(response: ProxiesResponse, name: string): ProxyNode | ProxyGroup | undefined {
  const candidates = proxyCandidates(response, name);
  // Group APIs expose names only. The first matching source depends on the
  // runtime group's provider/filter ordering, which is not exposed by this API.
  return candidates.length === 1 ? candidates[0].node : undefined;
}

export function catalogNodes(response: ProxiesResponse): ProxyCandidate[] {
  const nodes: ProxyCandidate[] = Object.values(response.proxies || {})
    .filter(node => !(node as ProxyNode)['provider-name'])
    .map(node => ({ node }));
  for (const [provider, data] of Object.entries(response.providers || {})) {
    if (data.vehicleType === 'Compatible') continue;
    nodes.push(...data.proxies.map(node => ({ provider, node })));
  }
  return nodes.filter(({ node }) => !groupTypes.includes(node.type) && !builtins.includes(node.type));
}
