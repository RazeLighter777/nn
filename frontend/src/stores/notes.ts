import { ref } from 'vue'
import { defineStore } from 'pinia'
import { notes } from '@/api/client'
import type { Note, NoteUpdate } from '@/api/types'
import type { NoteFilter } from '@/api/client'

export const useNotesStore = defineStore('notes', () => {
  const items = ref<Note[]>([])
  const total = ref(0)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchAll(params?: NoteFilter) {
    loading.value = true
    error.value = null
    try {
      const res = await notes.list(params)
      items.value = res.items
      total.value = res.total
    } catch (e) {
      error.value = (e as Error).message
    } finally {
      loading.value = false
    }
  }

  async function create(body: Note): Promise<Note | null> {
    try {
      const note = await notes.create(body)
      items.value.push(note)
      total.value++
      return note
    } catch (e) {
      error.value = (e as Error).message
      return null
    }
  }

  async function update(id: number, body: NoteUpdate): Promise<Note | null> {
    try {
      const updated = await notes.update(id, body)
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
      await notes.delete(id)
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
