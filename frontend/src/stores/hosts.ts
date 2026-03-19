import { ref } from 'vue'
import { defineStore } from 'pinia'
import { hosts } from '@/api/client'
import type { Host, HostUpdate } from '@/api/types'
import type { HostFilter } from '@/api/client'

export const useHostsStore = defineStore('hosts', () => {
  const items = ref<Host[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(params?: HostFilter) {
    loading.value = true
    error.value = null
    try {
      const res = await hosts.list(params)
      items.value = res.items
      total.value = res.total
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function create(body: Omit<Host, 'id'>): Promise<Host | null> {
    try {
      const host = await hosts.create(body)
      items.value.push(host)
      total.value++
      return host
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function update(id: number, body: HostUpdate): Promise<Host | null> {
    try {
      const updated = await hosts.update(id, body)
      const idx = items.value.findIndex((h) => h.id === id)
      if (idx !== -1) items.value[idx] = updated
      return updated
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function remove(id: number): Promise<boolean> {
    try {
      await hosts.delete(id)
      items.value = items.value.filter((h) => h.id !== id)
      total.value--
      return true
    } catch (e) {
      error.value = (e as Error).message
      return false
    }
  }

  return { items, total, loading, error, fetchAll, create, update, remove }
})
