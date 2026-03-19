import { ref } from 'vue'
import { defineStore } from 'pinia'
import { services } from '@/api/client'
import type { Service, ServiceUpdate } from '@/api/types'
import type { ServiceFilter } from '@/api/client'

export const useServicesStore = defineStore('services', () => {
  const items = ref<Service[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(params?: ServiceFilter) {
    loading.value = true
    error.value = null
    try {
      const res = await services.list(params)
      items.value = res.items
      total.value = res.total
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function create(body: Omit<Service, 'id'>): Promise<Service | null> {
    try {
      const svc = await services.create(body)
      items.value.push(svc)
      total.value++
      return svc
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function update(id: number, body: ServiceUpdate): Promise<Service | null> {
    try {
      const updated = await services.update(id, body)
      const idx = items.value.findIndex((s) => s.id === id)
      if (idx !== -1) items.value[idx] = updated
      return updated
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function remove(id: number): Promise<boolean> {
    try {
      await services.delete(id)
      items.value = items.value.filter((s) => s.id !== id)
      total.value--
      return true
    } catch (e) {
      error.value = (e as Error).message
      return false
    }
  }

  return { items, total, loading, error, fetchAll, create, update, remove }
})
