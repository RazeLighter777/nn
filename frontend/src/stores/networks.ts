import { ref } from 'vue'
import { defineStore } from 'pinia'
import { networks } from '@/api/client'
import type { Network, NetworkUpdate } from '@/api/types'
import type { NetworkFilter } from '@/api/client'

export const useNetworksStore = defineStore('networks', () => {
  const items = ref<Network[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(params?: NetworkFilter) {
    loading.value = true
    error.value = null
    try {
      const res = await networks.list(params)
      items.value = res.items
      total.value = res.total
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function create(body: Omit<Network, 'id'>): Promise<Network | null> {
    try {
      const net = await networks.create(body)
      items.value.push(net)
      total.value++
      return net
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function update(id: number, body: NetworkUpdate): Promise<Network | null> {
    try {
      const updated = await networks.update(id, body)
      const idx = items.value.findIndex((n) => n.id === id)
      if (idx !== -1) items.value[idx] = updated
      return updated
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function remove(id: number): Promise<boolean> {
    try {
      await networks.delete(id)
      items.value = items.value.filter((n) => n.id !== id)
      total.value--
      return true
    } catch (e) {
      error.value = (e as Error).message
      return false
    }
  }

  return { items, total, loading, error, fetchAll, create, update, remove }
})
