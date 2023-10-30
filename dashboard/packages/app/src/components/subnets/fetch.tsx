import { Operator, Subnet, Guest, NodeHealth, Rollout, Node, ChangePreview, SubnetUpdate } from './types';
import { useQuery } from 'react-query';
import { useApi, configApiRef } from '@backstage/core-plugin-api';

export function get_network() {
  const networkRegex = '/network/([^/]+)'
  return window.location.pathname.match(networkRegex)?.[1] ?? "mainnet"
}

export async function fetchVersion() {
  const network = get_network();
  const config = useApi(configApiRef);
  return fetch(
    `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/version`
  )
}

export function fetchOperators(): { [principal: string]: Operator } {
  const network = get_network();
  const config = useApi(configApiRef);
  const { data } = useQuery<{ [principal: string]: Operator }, Error>(`${network}_operators`, () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/operators`
    ).then((res) => res.json())
  );
  return data ?? {};
}

export function fetchNodes(): { [principal: string]: Node } {
  const network = get_network();
  const config = useApi(configApiRef);
  const { data } = useQuery<{ [principal: string]: Node }, Error>(`${network}_nodes`, () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/nodes`
    ).then((res) => res.json())
  );
  return data ?? {};
}

export function fetchSubnetVersions(): SubnetUpdate[] {
  const network = get_network();
  const config = useApi(configApiRef);
  const { data } = useQuery<SubnetUpdate[], Error>(`${network}_subnets_versions`, () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/subnets/versions`
    ).then((res) => res.json())
  );
  return data ?? [];
}

export function fetchMissingGuests(): Guest[] {
  const network = get_network();
  const config = useApi(configApiRef);
  const { data } = useQuery<Guest[], Error>("missing_guests", () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/missing_guests`
    ).then((res) => res.json())
  );
  return data ?? [];
}

export function fetchNodesHealths(): { [principal: string]: NodeHealth } {
  const network = get_network();
  const config = useApi(configApiRef);
  const { data } = useQuery<{ [principal: string]: NodeHealth }, Error>(`${network}_nodes_health`, () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/nodes/healths`
    ).then((res) => res.json())
  );
  return data ?? {};
}


export function fetchSubnets(): { [principal: string]: Subnet } {
  const network = get_network();
  const config = useApi(configApiRef);
  const { data: subnets } = useQuery<{ [principal: string]: Subnet }, Error>(`${network}_subnets`, () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/subnets`
    ).then((res) => res.json())
  );
  return subnets ?? {};
}

export function fetchRollouts(): Rollout[] {
  const network = get_network();
  const config = useApi(configApiRef);
  const { data: rollout } = useQuery<Rollout[], Error>(`${network}_rollout`, () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/rollout`
    ).then((res) => res.json())
  );
  return rollout ?? [];
}

export function fetchGuests(): Guest[] {
  const network = get_network();
  const config = useApi(configApiRef);
  const { data: guests } = useQuery<Guest[], Error>(`${network}_guests`, () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/guests`
    ).then((res) => res.json())
  );
  return guests ?? [];
}

export function fetchChangePreview(subnet: string): ChangePreview | undefined {
  const network = get_network();
  const config = useApi(configApiRef);
  const { data: change } = useQuery<ChangePreview, Error>(`${network}_change_preview_${subnet}`, () =>
    fetch(
      `${config.getString('backend.baseUrl')}/api/proxy/registry/${network}/subnet/${subnet}/change_preview`
    ).then((res) => res.json())
  );
  return change;
}
