// ---------------------------------------------------------------------------
// Typed API client for the Network Inventory API  (/api/v1)
// ---------------------------------------------------------------------------

import type {
  Site, SiteUpdate, SiteList,
  Tag, TagUpdate, TagList,
  Network, NetworkUpdate, NetworkList,
  Host, HostUpdate, HostList,
  Address, AddressUpdate, AddressList,
  Service, ServiceUpdate, ServiceList,
  Credential, CredentialUpdate, CredentialList,
  Note, NoteUpdate, NoteList, NoteCreate,
  TagAssignmentTarget, TagAssignmentList,
  CredentialServiceLink, CredentialServiceList,
  PaginationParams, SearchParams,
} from './types'

const BASE = '/api/v1'

// ---- HTTP helpers ---------------------------------------------------------

type Params = Record<string, string | number | boolean | null | undefined>

function buildUrl(path: string, params?: Params): string {
  const url = new URL(BASE + path, window.location.origin)
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      if (v !== null && v !== undefined && v !== '') {
        url.searchParams.set(k, String(v))
      }
    }
  }
  return url.toString()
}

async function request<T>(method: string, path: string, params?: Params, body?: unknown): Promise<T> {
  const url = buildUrl(path, params)
  const res = await fetch(url, {
    method,
    headers: body !== undefined ? { 'Content-Type': 'application/json' } : {},
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  if (res.status === 204) return undefined as T
  const data = await res.json()
  if (!res.ok) {
    throw new Error(data?.message ?? `HTTP ${res.status}`)
  }
  return data as T
}

const get  = <T>(path: string, params?: Params) => request<T>('GET',    path, params)
const post = <T>(path: string, body: unknown)   => request<T>('POST',   path, undefined, body)
const patch = <T>(path: string, body: unknown)  => request<T>('PATCH',  path, undefined, body)
const del  =     (path: string)                 => request<void>('DELETE', path)

// ---- Sites ----------------------------------------------------------------

export const sites = {
  list: (p?: SearchParams & { limit?: number; offset?: number }) =>
    get<SiteList>('/sites', p as Params),
  get: (id: number) => get<Site>(`/sites/${id}`),
  create: (body: Omit<Site, 'id'>) => post<Site>('/sites', body),
  update: (id: number, body: SiteUpdate) => patch<Site>(`/sites/${id}`, body),
  delete: (id: number) => del(`/sites/${id}`),
}

// ---- Tags -----------------------------------------------------------------

export const tags = {
  list: (p?: SearchParams) => get<TagList>('/tags', p as Params),
  get: (id: number) => get<Tag>(`/tags/${id}`),
  create: (body: Omit<Tag, 'id'>) => post<Tag>('/tags', body),
  update: (id: number, body: TagUpdate) => patch<Tag>(`/tags/${id}`, body),
  delete: (id: number) => del(`/tags/${id}`),
}

// ---- Networks -------------------------------------------------------------

export interface NetworkFilter extends SearchParams {
  site_id?: number
}

export const networks = {
  list: (p?: NetworkFilter) => get<NetworkList>('/networks', p as Params),
  get: (id: number) => get<Network>(`/networks/${id}`),
  create: (body: Omit<Network, 'id'>) => post<Network>('/networks', body),
  update: (id: number, body: NetworkUpdate) => patch<Network>(`/networks/${id}`, body),
  delete: (id: number) => del(`/networks/${id}`),
}

// ---- Hosts ----------------------------------------------------------------

export interface HostFilter extends SearchParams {
  site_id?: number
  os_type?: string
}

export const hosts = {
  list: (p?: HostFilter) => get<HostList>('/hosts', p as Params),
  get: (id: number) => get<Host>(`/hosts/${id}`),
  create: (body: Omit<Host, 'id'>) => post<Host>('/hosts', body),
  update: (id: number, body: HostUpdate) => patch<Host>(`/hosts/${id}`, body),
  delete: (id: number) => del(`/hosts/${id}`),
  listAddresses: (hostId: number, p?: PaginationParams & { ip_family?: number }) =>
    get<AddressList>(`/hosts/${hostId}/addresses`, p as Params),
  listServices: (hostId: number, p?: PaginationParams & { port?: number; state?: string }) =>
    get<ServiceList>(`/hosts/${hostId}/services`, p as Params),
}

// ---- Addresses ------------------------------------------------------------

export interface AddressFilter extends PaginationParams {
  host_id?: number
  network_id?: number
  ip?: string
  ip_family?: number
}

export const addresses = {
  list: (p?: AddressFilter) => get<AddressList>('/addresses', p as Params),
  get: (id: number) => get<Address>(`/addresses/${id}`),
  create: (body: Omit<Address, 'id'>) => post<Address>('/addresses', body),
  update: (id: number, body: AddressUpdate) => patch<Address>(`/addresses/${id}`, body),
  delete: (id: number) => del(`/addresses/${id}`),
}

// ---- Services -------------------------------------------------------------

export interface ServiceFilter extends SearchParams {
  site_id?: number
  address_id?: number
  port?: number
  state?: string
  ip_proto_number?: number
}

export const services = {
  list: (p?: ServiceFilter) => get<ServiceList>('/services', p as Params),
  get: (id: number) => get<Service>(`/services/${id}`),
  create: (body: Omit<Service, 'id'>) => post<Service>('/services', body),
  update: (id: number, body: ServiceUpdate) => patch<Service>(`/services/${id}`, body),
  delete: (id: number) => del(`/services/${id}`),
  listNotes: (serviceId: number, p?: PaginationParams) =>
    get<NoteList>(`/services/${serviceId}/notes`, p as Params),
  createNote: (serviceId: number, body: NoteCreate) =>
    post<Note>(`/services/${serviceId}/notes`, body),
  listTagAssignments: (serviceId: number, p?: PaginationParams) =>
    get<TagAssignmentList>(`/services/${serviceId}/tag-assignments`, p as Params),
  addTag: (serviceId: number, tagId: number) =>
    post<TagAssignmentTarget>(`/services/${serviceId}/tag-assignments`, { tag_id: tagId }),
  removeTag: (serviceId: number, assignmentId: number) =>
    del(`/services/${serviceId}/tag-assignments/${assignmentId}`),
}

// ---- Credentials ----------------------------------------------------------

export const credentials = {
  list: (p?: PaginationParams) => get<CredentialList>('/credentials', p as Params),
  get: (id: number) => get<Credential>(`/credentials/${id}`),
  create: (body: Omit<Credential, 'id'>) => post<Credential>('/credentials', body),
  update: (id: number, body: CredentialUpdate) => patch<Credential>(`/credentials/${id}`, body),
  delete: (id: number) => del(`/credentials/${id}`),
  listServices: (credId: number, p?: PaginationParams) =>
    get<CredentialServiceList>(`/credentials/${credId}/services`, p as Params),
  linkService: (credId: number, serviceId: number) =>
    post<CredentialServiceLink>(`/credentials/${credId}/services`, { service_id: serviceId }),
  unlinkService: (credId: number, linkId: number) =>
    del(`/credentials/${credId}/services/${linkId}`),
}

// ---- Notes ----------------------------------------------------------------

export interface NoteFilter extends PaginationParams {
  service_id?: number
  address_id?: number
  host_id?: number
  network_id?: number
  credential_id?: number
}

export const notes = {
  list: (p?: NoteFilter) => get<NoteList>('/notes', p as Params),
  get: (id: number) => get<Note>(`/notes/${id}`),
  create: (body: Note) => post<Note>('/notes', body),
  update: (id: number, body: NoteUpdate) => patch<Note>(`/notes/${id}`, body),
  delete: (id: number) => del(`/notes/${id}`),
}

// ---- Tag Assignments (flat) -----------------------------------------------

export interface TagAssignmentFilter extends PaginationParams {
  tag_id?: number
  service_id?: number
  address_id?: number
  host_id?: number
  network_id?: number
  credential_id?: number
}

export const tagAssignments = {
  list: (p?: TagAssignmentFilter) => get<TagAssignmentList>('/tag-assignments', p as Params),
  create: (body: Omit<TagAssignmentTarget, 'id'>) =>
    post<TagAssignmentTarget>('/tag-assignments', body),
  delete: (id: number) => del(`/tag-assignments/${id}`),
}

// ---- Credential-Service links (flat) -------------------------------------

export const credentialServices = {
  list: (p?: PaginationParams & { credential_id?: number; service_id?: number }) =>
    get<CredentialServiceList>('/credential-services', p as Params),
  create: (body: Omit<CredentialServiceLink, 'id'>) =>
    post<CredentialServiceLink>('/credential-services', body),
  delete: (id: number) => del(`/credential-services/${id}`),
}
