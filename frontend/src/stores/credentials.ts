import { ref } from 'vue'
import { defineStore } from 'pinia'
import { credentials } from '@/api/client'
import type { Credential, CredentialUpdate } from '@/api/types'

export const useCredentialsStore = defineStore('credentials', () => {
  const items = ref<Credential[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(params?: { limit?: number; offset?: number }) {
    loading.value = true
    error.value = null
    try {
      const res = await credentials.list(params)
      items.value = res.items
      total.value = res.total
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function create(body: Omit<Credential, 'id'>): Promise<Credential | null> {
    try {
      const cred = await credentials.create(body)
      items.value.push(cred)
      total.value++
      return cred
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function update(id: number, body: CredentialUpdate): Promise<Credential | null> {
    try {
      const updated = await credentials.update(id, body)
      const idx = items.value.findIndex((c) => c.id === id)
      if (idx !== -1) items.value[idx] = updated
      return updated
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function remove(id: number): Promise<boolean> {
    try {
      await credentials.delete(id)
      items.value = items.value.filter((c) => c.id !== id)
      total.value--
      return true
    } catch (e) {
      error.value = (e as Error).message
      return false
    }
  }

  return { items, total, loading, error, fetchAll, create, update, remove }
})
