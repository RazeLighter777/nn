import { ref } from 'vue'
import { defineStore } from 'pinia'
import { tags } from '@/api/client'
import type { Tag, TagUpdate } from '@/api/types'

export const useTagsStore = defineStore('tags', () => {
  const items = ref<Tag[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(params?: { limit?: number; offset?: number; q?: string }) {
    loading.value = true
    error.value = null
    try {
      const res = await tags.list(params)
      items.value = res.items
      total.value = res.total
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function create(body: Omit<Tag, 'id'>): Promise<Tag | null> {
    try {
      const tag = await tags.create(body)
      items.value.push(tag)
      total.value++
      return tag
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function update(id: number, body: TagUpdate): Promise<Tag | null> {
    try {
      const updated = await tags.update(id, body)
      const idx = items.value.findIndex((t) => t.id === id)
      if (idx !== -1) items.value[idx] = updated
      return updated
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function remove(id: number): Promise<boolean> {
    try {
      await tags.delete(id)
      items.value = items.value.filter((t) => t.id !== id)
      total.value--
      return true
    } catch (e) {
      error.value = (e as Error).message
      return false
    }
  }

  return { items, total, loading, error, fetchAll, create, update, remove }
})
