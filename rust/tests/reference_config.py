"""Extract unchanged C++ configuration code for compatibility tests (no HID/input).

python rust/tests/reference_config.py <output.cpp>
Compile with MSVC /std:c++20 /EHsc /utf-8 and /I <repository>/src.
--generate FILE writes the public fixture; --check FILE validates;
--roundtrip INPUT OUTPUT uses the original C++ reader and writer.
"""
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[2]
source = (root / 'src/app/main.cpp').read_text()
codec = (root / 'src/app/SettingsJson.inc').read_text()
def part(start, end):
    return source[source.index(start):source.index(end)]

header = r'''
#include <Windows.h>
#include <array>
#include <atomic>
#include <cstdint>
#include <cwchar>
#include <limits>
#include <string>
#include <vector>
#include <iostream>
#include <chrono>
#include <mutex>
#include <algorithm>
#include <Shellapi.h>
#include <cwctype>
#include <bcrypt.h>
#include <fstream>
#include <filesystem>
#include "app/UpdateProtocol.h"
#include "app/SimpleJson.h"
#include "app/TargetIcon.cpp"
'''
models = part('constexpr uint32_t EXTENDED', 'struct ButtonDef {')
models += part('struct TurboSettings {', 'std::vector<Layer> liveLayers;')
models += part('struct Preset {', 'enum class RowClipboardKind')
models += part('struct ButtonDef {', 'constexpr size_t VISIBLE_ROWS')
models += part('enum class RowClipboardKind', 'RowClipboard rowClipboard;')
globals_code = r'''
std::wstring language = L"ja";
uint32_t closeBehavior = 2;
std::atomic<bool> autoMode{true}, steamTakeover{true};
size_t selectedPreset = 0;
std::vector<std::wstring> folders;
std::vector<Preset> presets;
using Clock = std::chrono::steady_clock;
std::atomic<uint64_t> targetVersion{0};
std::atomic<bool> requested{false}, previewMode{false};
std::mutex bindingMutex;
std::wstring liveTargetExecutable;
HWND mainWindow = nullptr;
bool processElevated = false;
bool webReady = true;
int editedLayer = 0;
std::wstring latestStatus;
struct ReferenceWebUi {
    std::wstring message;
    std::vector<std::wstring> messages;
    void SendJson(const std::wstring& json) { message=json;messages.push_back(json); }
} webUi;
RowClipboard rowClipboard;
std::wstring settingsPath,referenceImportPath;
bool referenceDiskSave=false;
std::wstring ConfigJson();
bool WriteUtf8File(const std::wstring&,const std::wstring&);
bool referenceSaveOkay=true;
bool referenceImportFinalFail=false;
unsigned int referenceSaveCalls=0;
uint64_t referenceGeneration=0;
bool SavePresets() { if(referenceDiskSave)return WriteUtf8File(settingsPath,ConfigJson()); return referenceSaveOkay && !(referenceImportFinalFail && ++referenceSaveCalls==2); }
void PublishSelectedPreset() { ++referenceGeneration; }
void UpdatePresetBox() {}
void UpdateLayerControls() {}
void UpdateTargetButton() {}
HWND toggleButton=nullptr;
constexpr UINT WM_UI_READY=WM_APP+5,WM_FORCE_EXIT=WM_APP+4;
bool HideToTray(HWND) {return false;}
bool StartUpdate(const std::wstring&) {return false;}
std::wstring TargetIconCacheDirectory() {return L"";}
std::wstring referencePickedTarget;
bool SetTargetExecutable(const std::wstring& path);
void PickTargetExecutable(HWND) {
    if(referencePickedTarget!=L"-")SetTargetExecutable(referencePickedTarget);
}
HWND autoModeBox=nullptr;
bool ChooseFile(HWND,bool,const wchar_t*,const wchar_t*,wchar_t (&path)[MAX_PATH*2]) {
    if(referenceImportPath.empty())return false;
    wcscpy_s(path,referenceImportPath.c_str());return true;
}
int ReferenceMessageBox(HWND,LPCWSTR,LPCWSTR,UINT) {return IDOK;}
#define MessageBoxW ReferenceMessageBox
void ImportSettings(HWND);
void ExportSettings(HWND) {}
bool SetTargetExecutable(const std::wstring& path) {
    auto& target=presets[selectedPreset].targetExecutable;
    auto previous=target;target=path;
    if(!SavePresets()){target=std::move(previous);return false;}
    PublishSelectedPreset();return true;
}
'''
functions = part('uint32_t ReadBinding(', 'void PublishSelectedPreset()')
functions += part('void LoadLegacyPresets()', '#include "SettingsJson.inc"')
functions += codec[codec.index('std::wstring ConfigQuote'):codec.index('bool SavePresets()')]
functions += codec[codec.index('const simple_json::Value* ConfigField'):codec.index('void ApplyConfig')]
functions += part('bool IsTargetForeground() {', 'struct DeviceCycleProcess {')
functions += codec[codec.index('void ApplyConfig'):]
functions += part('bool SetBinding(size_t button', 'void UpdatePresetBox()')
functions += part('bool SelectPreset(size_t index)', 'void UpdateRoleControls()')
functions += part('void ImportSettings(HWND window)', 'void ShowPopup(HWND window')
bridge = (root / 'src/app/WebBridge.inc').read_text()
functions += bridge
updater = (root / 'src/app/Updater.cpp').read_text()
functions += updater[updater.index('bool Utf8ToWide('):updater.index('bool WriteFileBytes(')]
functions += '\nnamespace fs = std::filesystem;\n'
functions += updater[updater.index('bool ApplyPackage('):updater.index('void NotifyFailure(')]
main = r'''
int wmain(int argc, wchar_t** argv) {
    if(argc<3)return 2;
    const std::wstring mode=argv[1];
    if(mode==L"--apply-package" && argc==5)return ApplyPackage(argv[2],argv[3],argv[4])?0:1;
    if(mode==L"--sha256" && argc==4) {
        std::ifstream input(std::filesystem::path(argv[2]),std::ios::binary);
        std::vector<unsigned char> bytes((std::istreambuf_iterator<char>(input)),{});
        std::string digest;
        if(!Sha256(bytes,digest))return 3;
        return WriteUtf8File(argv[3],std::wstring(digest.begin(),digest.end()))?0:3;
    }
    if(mode==L"--validate-tag" && argc==3)return ValidUpdateTag(argv[2])?0:1;
    if(mode==L"--release-digest" && argc==5) {
        std::ifstream input(std::filesystem::path(argv[2]),std::ios::binary);
        std::vector<unsigned char> bytes((std::istreambuf_iterator<char>(input)),{});
        std::string digest;
        bool okay=ReleaseDigest(bytes,argv[3],digest);
        return WriteUtf8File(argv[4],okay?ConfigQuote(std::wstring(digest.begin(),digest.end())):L"null")?0:3;
    }
    if((mode==L"--legacy" && argc==4) || (mode==L"--startup" && argc==3)) {
        language=L"ja";closeBehavior=0;autoMode=false;steamTakeover=false;
        settingsPath=argv[2];referenceDiskSave=true;
        if(mode==L"--legacy") {
            LoadLegacyPresets();
            return WriteUtf8File(argv[3],ConfigJson())?0:3;
        }
        const auto folder=settingsPath.substr(0,settingsPath.find_last_of(L"\\/"));
        CreateDirectoryW(folder.c_str(),nullptr);
        return LoadSettings()?0:1;
    }
    if(mode==L"--icon" && argc==5) {
        if(FAILED(CoInitializeEx(nullptr,COINIT_APARTMENTTHREADED)))return 3;
        const auto icon=TargetIconDataUri(argv[2],argv[3]);
        CoUninitialize();
        return WriteUtf8File(argv[4],icon)?0:3;
    }
    if(mode==L"--decode" && argc==4) {
        const auto fields=DecodeMessage(argv[2]);
        std::wstring json=L"[";
        for(size_t i=0;i<fields.size();++i) {
            if(i)json+=L",";
            json+=JsonString(fields[i]);
        }
        return WriteUtf8File(argv[3],json+L"]") ? 0 : 3;
    }
    if(mode==L"--generate" && argc==3) {
        folders={L"分類"};
        Preset preset;
        preset.name=L"テスト\U0001f600 \"\\\t";
        preset.folder=folders[0];
        preset.targetExecutable=L"C:\\Games\\Example.exe";
        preset.leftPadMode=2; preset.rightPadMode=1;
        preset.leftPadSensitivity=25; preset.rightPadSensitivity=400;
        preset.leftStickDeadzone=0; preset.rightStickDeadzone=32767;
        preset.leftStickOverlap=0; preset.rightStickOverlap=32767;
        preset.mapping[A]=0x1e; preset.mapping[B]=0x10048;
        preset.mapping[LB]=MOUSE_LEFT;
        for(size_t index=DPadUp;index<=DPadRight;++index)
            preset.mapping[index]=static_cast<uint32_t>(0x20+index);
        preset.turbo[A]={true,20,5000};
        preset.sequence[B]={true,false,2000,{0x1e,0x10048,MOUSE_MIDDLE}};
        preset.sequence[DPadUp]={true,true,50,{0x1e}};
        Layer layer;
        layer.name=L"レイヤー\U0001f600";
        layer.trigger=ButtonCount;
        layer.mapping[X]=MOUSE_RIGHT;
        layer.sequence[Y]={true,true,20,{0x1004b}};
        preset.layers.push_back(layer);
        presets.push_back(preset);
        return WriteUtf8File(argv[2],ConfigJson()) ? 0 : 3;
    }
    if(mode==L"--number" && argc==4) {
        uint32_t number=0;
        return WriteUtf8File(argv[3],ParseNumber(argv[2],number)?std::to_wstring(number):L"null") ? 0:3;
    }
    if(mode==L"--number-spaces" && argc==3) {
        std::wstring json=L"[";bool first=true;
        for(uint32_t ch=1;ch<65536;++ch)if(std::iswspace(static_cast<wint_t>(ch))){
            if(!first)json+=L",";
            first=false;json+=std::to_wstring(ch);
        }
        return WriteUtf8File(argv[2],json+L"]")?0:3;
    }
    if(mode==L"--placement" && argc==4) {
        int left=0,top=0,right=0,bottom=0,maximized=0;
        const bool okay=swscanf_s(argv[2],L"%d,%d,%d,%d,%d",&left,&top,&right,&bottom,&maximized)==5;
        return WriteUtf8File(argv[3],okay?L"["+std::to_wstring(left)+L","+std::to_wstring(top)+L","+std::to_wstring(right)+L","+std::to_wstring(bottom)+L","+std::to_wstring(maximized)+L"]":L"null") ? 0:3;
    }
    ConfigSnapshot config;
    const bool accepted=ReadConfig(argv[2],config);
    if(mode==L"--foreground" && argc==3) {
        if(!accepted)return 1;
        liveTargetExecutable=config.presets[config.selectedPreset].targetExecutable;
        autoMode=config.autoMode;
        std::cout<<"selected_target_foreground="<<(IsTargetForeground() ? "true" : "false")
                 <<", automatic_active="<<(ShouldHoldController() ? "true" : "false")<<"\n";
        return 0;
    }
    if(mode==L"--check" && argc==3) {
        std::cout<<(accepted ? "accepted" : "rejected")<<"\n";
        return accepted ? 0 : 1;
    }
    if((mode!=L"--roundtrip" && mode!=L"--ui-state" && mode!=L"--ui-commands") || argc!=4)return 2;
    if(!accepted)return 1;
    language=std::move(config.language);closeBehavior=config.closeBehavior;
    autoMode=config.autoMode;steamTakeover=config.steamTakeover;
    selectedPreset=config.selectedPreset;folders=std::move(config.folders);
    presets=std::move(config.presets);
    if(mode==L"--ui-commands") {
        settingsPath=argv[2];
        std::wstring commands;
        if(!ReadUtf8File(argv[3],commands))return 3;
        std::wstring output=L"[";
        size_t start=0;
        while(start<commands.size()) {
            size_t end=commands.find(L'\n',start);
            auto command=commands.substr(start,end==std::wstring::npos?end:end-start);
            if(!command.empty()&&command.back()==L'\r')command.pop_back();
            referenceSaveOkay=!command.starts_with(L"FAIL\t");
            if(!referenceSaveOkay)command.erase(0,5);
            referenceImportFinalFail=command.starts_with(L"IMPORTFAIL\t");
            referenceSaveCalls=0;
            if(command.starts_with(L"IMPORT\t") || referenceImportFinalFail) {
                auto file=command.substr(referenceImportFinalFail?11:7);
                referenceImportPath=file==L"-"?L"":settingsPath.substr(0,settingsPath.find_last_of(L"\\/")+1)+file;
                command=L"import";
            }
            if(command.starts_with(L"PICK\t")) {
                referencePickedTarget=command.substr(5);command=L"pickTarget";
            }
            webUi.messages.clear();OnWebMessage(command);
            if(start)output+=L",";
            output+=L"{\"config\":"+ConfigJson()+L",\"editedLayer\":"+std::to_wstring(editedLayer)+
                L",\"requested\":"+JsonBool(requested)+L",\"preview\":"+JsonBool(previewMode)+
                L",\"generation\":"+std::to_wstring(referenceGeneration)+L",\"messages\":[";
            for(size_t i=0;i<webUi.messages.size();++i){if(i)output+=L",";output+=webUi.messages[i];}
            output+=L"]}";
            if(end==std::wstring::npos)break;
            start=end+1;
        }
        const std::wstring outputFile=std::wstring(argv[3])+L".output.json";
        return WriteUtf8File(outputFile,output+L"]")?0:3;
    }
    if(mode==L"--ui-state") {
        SendUiState();
        return WriteUtf8File(argv[3],webUi.message) ? 0 : 3;
    }
    return WriteUtf8File(argv[3],ConfigJson()) ? 0 : 3;
}
'''
Path(sys.argv[1]).write_text(header + models + globals_code + functions + main)
