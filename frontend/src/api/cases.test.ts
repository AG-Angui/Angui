import { describe, expect, it, vi } from 'vitest'
import {
  addCaseMember,
  createCase,
  createClue,
  createSummaryDraft,
  getCase,
  listCaseClues,
  listCases,
  reviewClue,
  updateCaseStatus,
} from './cases'

function jsonResponse(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

function requestOptions(call: unknown[]) {
  return call[1] as RequestInit
}

describe('case API contract', () => {
  it('uses the documented collection and detail endpoints', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse(200, []))
      .mockResolvedValueOnce(jsonResponse(200, { id: 'case-1' }))
    vi.stubGlobal('fetch', fetchMock)

    await listCases('test-session')
    await getCase('test-session', 'case-1')

    expect(fetchMock.mock.calls.map(([path]) => path)).toEqual(['/api/cases', '/api/cases/case-1'])
  })

  it('passes queue filters and pagination to the documented clue timeline endpoint', async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { items: [], page: 2, page_size: 25, total: 26 }))
    vi.stubGlobal('fetch', fetchMock)

    await listCaseClues('test-session', 'case-1', {
      page: 2,
      page_size: 25,
      status: 'pending_review',
      source_type: 'field_report',
      sort: 'occurred_at',
      order: 'asc',
    })

    expect(fetchMock.mock.calls[0][0]).toBe('/api/cases/case-1/clues?page=2&page_size=25&status=pending_review&source_type=field_report&sort=occurred_at&order=asc')
  })

  it('sends a documented case creation request', async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(201, { id: 'case-1' }))
    vi.stubGlobal('fetch', fetchMock)

    await createCase('test-session', {
      display_name: '模拟老人 A',
      age: 76,
      gender: null,
      physical_description: null,
      clothing_description: null,
      health_notes: null,
      last_seen_at: null,
      last_seen_location: '模拟公园北门',
    })

    const [path, options] = fetchMock.mock.calls[0]
    expect(path).toBe('/api/cases')
    expect(requestOptions([path, options]).method).toBe('POST')
    expect(JSON.parse(String(requestOptions([path, options]).body))).toMatchObject({
      display_name: '模拟老人 A',
      last_seen_location: '模拟公园北门',
    })
  })

  it('sends member, clue, review, and status mutations to their documented endpoints', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse(201, { user_id: 'member-1' }))
      .mockResolvedValueOnce(jsonResponse(201, { id: 'clue-1' }))
      .mockResolvedValueOnce(jsonResponse(200, { id: 'clue-1', status: 'confirmed' }))
      .mockResolvedValueOnce(jsonResponse(200, { id: 'case-1', status: 'resolved' }))
    vi.stubGlobal('fetch', fetchMock)

    await addCaseMember('test-session', 'case-1', 'volunteer@demo.invalid', 'volunteer')
    await createClue('test-session', 'case-1', {
      source: 'volunteer',
      content: '模拟线索',
      occurred_at: null,
      location_text: null,
    })
    await reviewClue('test-session', 'clue-1', { status: 'confirmed', reason: 'Reviewed source record' })
    await updateCaseStatus('test-session', 'case-1', 'resolved')

    expect(fetchMock.mock.calls.map(([path]) => path)).toEqual([
      '/api/cases/case-1/members',
      '/api/cases/case-1/clues',
      '/api/clues/clue-1/review',
      '/api/cases/case-1/status',
    ])
    expect(fetchMock.mock.calls.map(requestOptions).map((options) => options.method)).toEqual([
      'POST',
      'POST',
      'PATCH',
      'PATCH',
    ])
    expect(
      fetchMock.mock.calls.map((call) => JSON.parse(String(requestOptions(call).body))),
    ).toEqual([
      { email: 'volunteer@demo.invalid', case_role: 'volunteer' },
      { source: 'volunteer', content: '模拟线索', occurred_at: null, location_text: null },
      { status: 'confirmed', reason: 'Reviewed source record' },
      { status: 'resolved' },
    ])
  })

  it('preserves an explicit empty summary draft content so the server can reject it', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse(201, { id: 'summary-1' }))
      .mockResolvedValueOnce(jsonResponse(400, { error: { code: 'validation_error', message: 'content is required' } }))
    vi.stubGlobal('fetch', fetchMock)

    await createSummaryDraft('test-session', 'case-1')
    await expect(createSummaryDraft('test-session', 'case-1', '')).rejects.toMatchObject({ status: 400 })

    expect(fetchMock.mock.calls.map((call) => JSON.parse(String(requestOptions(call).body)))).toEqual([
      {},
      { content: '' },
    ])
  })
})
