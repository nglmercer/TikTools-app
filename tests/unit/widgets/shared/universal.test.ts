import {test,expect} from 'vitest';
import cases from '../../../../src/universal/fixtures/conformance.json';
import {validateEvent} from '../../../../src/universal';
import {EditorSession,portableProject,exportPreset,importPreset} from '../../../../src/universal/editor.js';
test('vendored canonical fixtures match native and browser contracts',()=>{for(const fixture of cases){let valid=true;try{validateEvent(fixture.event);}catch{valid=false;}expect(valid).toBe(fixture.valid);}});
test('portable presets use the shared history and canonical serialization',()=>{const project=portableProject({id:'universal.gift-alert',version:'1.0.0'},{title:'TikTools'});const editor=new EditorSession(project);editor.execute({type:'settings',value:{title:'OBS'}});const preset=JSON.parse(exportPreset(editor.project));const imported=importPreset(project,preset);expect(imported.project.settings.title).toBe('OBS');editor.undo();expect(editor.project.settings.title).toBe('TikTools');});
