// ---------------------------------------------------------------------------
// TypeScript types generated from spec.txt (OpenAPI 3.1)
// ---------------------------------------------------------------------------

// ---- Core resources -------------------------------------------------------

export interface Site {
  id?: number
  name: string
}

export interface Tag {
  id?: number
  name: string
}

export interface Network {
  id?: number
  site_id: number
  name: string
}

export interface Host {
  id?: number
  site_id: number
  name: string
  os_type?: string | null
  hostname?: string | null
}

export interface Address {
  id?: number
  host_id: number
  network_id: number
  ip: string
  ip_family: 4 | 6
  netmask: number
  mac?: string | null
}

export interface Service {
  id?: number
  site_id: number
  address_id: number
  port: number
  ip_proto_number: number
  state: string
  name: string
  product?: string | null
  version?: string | null
  extra_info?: string | null
  os_type?: string | null
  device_type?: string | null
  hostname?: string | null
  confidence?: number | null
  method?: string | null
  service_fp?: string | null
  cpe?: string | null
  rpcnum?: number | null
  lowver?: number | null
  highver?: number | null
  owner?: string | null
}

export interface Credential {
  id?: number
  username?: string | null
  password?: string | null
  hash?: string | null
}

export interface Note {
  id?: number
  text: string
  service_id?: number | null
  address_id?: number | null
  host_id?: number | null
  network_id?: number | null
  credential_id?: number | null
}

// ---- Association types ----------------------------------------------------

export interface TagAssignmentTarget {
  id?: number
  tag_id: number
  service_id?: number | null
  address_id?: number | null
  host_id?: number | null
  network_id?: number | null
  credential_id?: number | null
}

export interface CredentialServiceLink {
  id?: number
  credential_id: number
  service_id: number
}

// ---- Update (PATCH) bodies — all fields optional -------------------------

export type SiteUpdate = Partial<Pick<Site, 'name'>>

export type TagUpdate = Partial<Pick<Tag, 'name'>>

export type NetworkUpdate = Partial<Pick<Network, 'site_id' | 'name'>>

export type HostUpdate = Partial<Pick<Host, 'site_id' | 'name' | 'os_type' | 'hostname'>>

export type AddressUpdate = Partial<
  Pick<Address, 'host_id' | 'network_id' | 'ip' | 'ip_family' | 'netmask' | 'mac'>
>

export type ServiceUpdate = Partial<
  Omit<Service, 'id'>
>

export type CredentialUpdate = Partial<
  Pick<Credential, 'username' | 'password' | 'hash'>
>

export type NoteUpdate = Partial<
  Pick<Note, 'text' | 'service_id' | 'address_id' | 'host_id' | 'network_id' | 'credential_id'>
>

// ---- Sub-resource creation bodies ----------------------------------------

export interface NoteCreate {
  text: string
}

// ---- List envelopes -------------------------------------------------------

export interface ListResponse<T> {
  total: number
  items: T[]
}

export type SiteList = ListResponse<Site>
export type TagList = ListResponse<Tag>
export type NetworkList = ListResponse<Network>
export type HostList = ListResponse<Host>
export type AddressList = ListResponse<Address>
export type ServiceList = ListResponse<Service>
export type CredentialList = ListResponse<Credential>
export type NoteList = ListResponse<Note>
export type TagAssignmentList = ListResponse<TagAssignmentTarget>
export type CredentialServiceList = ListResponse<CredentialServiceLink>

// ---- Error ----------------------------------------------------------------

export interface ApiError {
  code?: number
  message: string
}

// ---- Common query params --------------------------------------------------

export interface PaginationParams {
  limit?: number
  offset?: number
}

export interface SearchParams extends PaginationParams {
  q?: string
}
