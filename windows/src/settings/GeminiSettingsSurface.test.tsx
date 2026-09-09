import {act,fireEvent,render,screen} from '@testing-library/react'
import {it,expect,vi,afterEach} from 'vitest'
const backend=vi.hoisted(()=>({listeners:new Map<string,(event:{payload:unknown})=>void>(), fresh:false, checks:[] as boolean[]}))
vi.mock('@tauri-apps/api/window',()=>({getCurrentWindow:()=>({label:'settings'})}))
vi.mock('@tauri-apps/api/event',()=>({listen:async(name:string,fn:(event:{payload:unknown})=>void)=>{backend.listeners.set(name,fn);return()=>backend.listeners.delete(name)}}))
vi.mock('@tauri-apps/api/core',()=>({invoke:async(command:string,args?:{providerId?:string,retryUsage?:boolean})=>{
 if(command==='app_settings')return {}
 if(command==='update_state')return {phase:'idle',currentVersion:'0.5.1'}
 if(command==='service_account_status'){
  if(args?.providerId==='gemini')backend.checks.push(args.retryUsage??false)
  return {providerId:args?.providerId,connectionState:backend.fresh&&args?.providerId==='gemini'?'connected':'unavailable',accountDetail:args?.providerId==='gemini'?(backend.fresh?'Official Gemini quota available':'Gemini not yet checked'):undefined}
 }
 return []
}}))
import {Shell} from '../Shell'
afterEach(()=>{Reflect.deleteProperty(window,'__TAURI_INTERNALS__')})
it('open Settings follows the same Gemini collector result without launching another refresh',async()=>{
 Object.defineProperty(window,'__TAURI_INTERNALS__',{value:{},configurable:true})
 render(<Shell/>);fireEvent.click(screen.getByRole('tab',{name:'Services'}))
 expect(await screen.findByText('Gemini not yet checked')).toBeVisible()
 backend.fresh=true
 await act(async()=>{backend.listeners.get('snapshot-updated')?.({payload:{providerId:'gemini',status:'fresh'}})})
 expect(await screen.findByText('Official Gemini quota available')).toBeVisible()
 expect(backend.checks).toEqual([false,false])
})
