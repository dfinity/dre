import { Operator, Subnet, Host, NodeHealth, Rollout, Node } from './types';
import { useQuery } from 'react-query';
import { useApi, configApiRef } from '@backstage/core-plugin-api';



export function fetchOperators(): { [principal: string]: Operator } {
    const config = useApi(configApiRef);
    const { data } = useQuery<{ [principal: string]: Operator }, Error>("operators", () =>
        fetch(
            `${config.getString('backend.baseUrl')}/api/proxy/registry/operators`
        ).then((res) => res.json())
    );
    return data ?? {};
}

export function fetchNodes(): { [principal: string]: Node } {
    const config = useApi(configApiRef);
    const { data } = useQuery<{ [principal: string]: Node }, Error>("nodes", () =>
        fetch(
            `${config.getString('backend.baseUrl')}/api/proxy/registry/nodes`
        ).then((res) => res.json())
    );
    return data ?? {};
}

export function fetchMissingHosts(): Host[] {
    const config = useApi(configApiRef);
    const { data } = useQuery<Host[], Error>("missing_hosts", () =>
        fetch(
            `${config.getString('backend.baseUrl')}/api/proxy/registry/missing_hosts`
        ).then((res) => res.json())
    );
    return data ?? [];
}

export function fetchNodesHealths(): { [principal: string]: NodeHealth } {
    const config = useApi(configApiRef);
    const { data } = useQuery<{ [principal: string]: NodeHealth }, Error>("nodes_health", () =>
        fetch(
            `${config.getString('backend.baseUrl')}/api/proxy/registry/nodes/healths`
        ).then((res) => res.json())
    );
    return data ?? {};
}


export function fetchSubnets(): { [principal: string]: Subnet } {
    const config = useApi(configApiRef);
    const { data: subnets } = useQuery<{ [principal: string]: Subnet }, Error>("subnets", () =>
        fetch(
            `${config.getString('backend.baseUrl')}/api/proxy/registry/subnets`
        ).then((res) => res.json())
    );
    return subnets ?? {};
}

export function fetchRollout(): Rollout {
    const config = useApi(configApiRef);
    const { data: rollout } = useQuery<Rollout, Error>("rollout", () =>
        fetch(
            `${config.getString('backend.baseUrl')}/api/proxy/registry/rollout`
        ).then((res) => res.json())
    );
    return rollout ?? {
        stages: [],
        release: {
            name: "",
            branch: "",
            time: "",
            commit_hash: "",
        },
    };
}

export function fetchHosts(): Host[] {
    const config = useApi(configApiRef);
    const { data: hosts } = useQuery<Host[], Error>("hosts", () =>
        fetch(
            `${config.getString('backend.baseUrl')}/api/proxy/registry/hosts`
        ).then((res) => res.json())
    );
    return hosts ?? [];
}
