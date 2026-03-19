import { ref } from 'vue'
import { defineStore } from 'pinia'
import { sites } from '@/api/client'
import type { Site, SiteUpdate } from '@/api/types'

export const useSitesStore = defineStore('sites', () => {
  const items = ref<Site[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(params?: { limit?: number; offset?: number; q?: string }) {
    loading.value = true
    error.value = null
    try {
      const res = await sites.list(params)
      items.value = res.items
      total.value = res.total
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function create(body: Omit<Site, 'id'>): Promise<Site | null> {
    try {
      const site = await sites.create(body)
      items.value.push(site)
      total.value++
      return site
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function update(id: number, body: SiteUpdate): Promise<Site | null> {
    try {
      const updated = await sites.update(id, body)
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
      await sites.delete(id)
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
