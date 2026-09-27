cmake_minimum_required(VERSION 3.20)
project(Remapcon VERSION 1.0.0 LANGUAGES CXX)

include(FetchContent)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_library(SteamController STATIC
    src/hid/HidDevice.cpp
    src/steam/SteamController.cpp
)
target_include_directories(SteamController PUBLIC src)
target_link_libraries(SteamController PUBLIC hid setupapi)
if(MSVC)
    target_compile_options(SteamController PRIVATE /W4 /WX /utf-8)
endif()

FetchContent_Declare(WebView2
    URL https://www.nuget.org/api/v2/package/Microsoft.Web.WebView2/1.0.2849.39
    DOWNLOAD_EXTRACT_TIMESTAMP ON
)
FetchContent_MakeAvailable(WebView2)
if(CMAKE_SIZEOF_VOID_P EQUAL 8)
    set(WV2_ARCH x64)
else()
    set(WV2_ARCH x86)
endif()

set(APP_UI_DIR "${CMAKE_CURRENT_SOURCE_DIR}/src/app/ui")
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS
    "${APP_UI_DIR}/index.html" "${APP_UI_DIR}/style.css" "${APP_UI_DIR}/app.js"
    "${CMAKE_CURRENT_SOURCE_DIR}/src/app/resources/remapcon.svg")
file(READ "${APP_UI_DIR}/index.html" APP_UI_HTML)
file(READ "${APP_UI_DIR}/style.css" APP_UI_CSS)
file(READ "${APP_UI_DIR}/app.js" APP_UI_JS)
file(READ "${CMAKE_CURRENT_SOURCE_DIR}/src/app/resources/remapcon.svg" APP_BRAND_SVG)
string(REPLACE "<!-- STYLE -->" "<style>${APP_UI_CSS}</style>" APP_UI_HTML "${APP_UI_HTML}")
string(REPLACE "<!-- SCRIPT -->" "<script>${APP_UI_JS}</script>" APP_UI_HTML "${APP_UI_HTML}")
string(REPLACE "<!-- BRAND_ICON -->" "${APP_BRAND_SVG}" APP_UI_HTML "${APP_UI_HTML}")
string(REPLACE "\n" "\n)MAPLEUI\",\n    LR\"MAPLEUI(" MAPLE_UI_HTML "${APP_UI_HTML}")
configure_file(src/app/ui_embedded.h.in "${CMAKE_CURRENT_BINARY_DIR}/maple_ui_embedded.h" @ONLY)

add_executable(Remapcon WIN32
    src/app/main.cpp
    src/app/SettingsJson.inc
    src/app/SimpleJson.h
    src/app/WebBridge.inc
    src/app/TargetIcon.cpp
    src/app/WebUi.cpp
    src/app/resources/app.rc
)
target_include_directories(Remapcon PRIVATE
    "${CMAKE_CURRENT_BINARY_DIR}"
    "${webview2_SOURCE_DIR}/build/native/include")
target_link_libraries(Remapcon PRIVATE
    SteamController user32 shell32 comdlg32 ole32 dwmapi advapi32 windowscodecs
    "${webview2_SOURCE_DIR}/build/native/${WV2_ARCH}/WebView2LoaderStatic.lib")
if(MSVC)
    target_compile_options(Remapcon PRIVATE /W4 /WX /utf-8)
endif()

install(TARGETS Remapcon RUNTIME DESTINATION .)
install(FILES LICENSE README.md README.ja.md THIRD_PARTY_NOTICES.md DESTINATION .)
install(FILES LICENSES/WebView2-LICENSE.txt LICENSES/WebView2-NOTICE.txt
    DESTINATION LICENSES)

set(CPACK_GENERATOR ZIP)
set(CPACK_PACKAGE_NAME Remapcon)
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_FILE_NAME "Remapcon-Windows-${WV2_ARCH}")
include(CPack)
