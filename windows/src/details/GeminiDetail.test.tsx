import { render,screen } from '@testing-library/react'
import { expect,it } from 'vitest'
import {ProviderDetail} from './ProviderDetail'
import type {UsageSnapshot} from '../state/usage'
import fresh from '../../../contracts/fixtures/gemini-fresh.json'
const handlers={onPointerEnter:()=>{},onPointerLeave:()=>{},onInteractionStart:()=>{},onInteractionEnd:()=>{}}
it('renders every official visible tier, source and original reset text even when not primary or secondary',()=>{
 const snapshot={...fresh,geminiQuotaMetrics:[...fresh.geminiQuotaMetrics,{...fresh.geminiQuotaMetrics[0],label:'Flash Lite',current:10,resetDescription:null}]} as UsageSnapshot
 render(<ProviderDetail {...handlers} snapshot={snapshot}/> )
 for(const label of ['Pro','Flash','Flash Lite'])expect(screen.getByText(label)).toBeVisible()
 expect(screen.getByText('10%')).toBeVisible()
 expect(screen.getByText(/Gemini CLI 0.58.0.*\/model/)).toBeVisible()
 expect(screen.getAllByText('Resets: 5:47 PM (1h)')).toHaveLength(2)
})
it('cached failures retain tier data and show the reason rather than claiming fresh account access',()=>{
 render(<ProviderDetail {...handlers} snapshot={{...fresh,status:'cached',statusMessage:'Cached · sign in required'} as UsageSnapshot}/> )
 expect(screen.getByText('Cached · sign in required')).toBeVisible()
 expect(screen.getByText('Pro')).toBeVisible()
 expect(screen.getByText('Flash')).toBeVisible()
})
