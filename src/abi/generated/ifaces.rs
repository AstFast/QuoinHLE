// @generated from local ABI definitions. DO NOT EDIT.
// Re-run the generator instead of patching this file.
#![allow(dead_code, non_upper_case_globals, non_snake_case)]
use crate::abi::{IfaceInfo, MethodInfo};
use super::meta::NONE_IID;

pub static IFACES: &[IfaceInfo] = &[
    IfaceInfo {
        name: "IAClockCtl",
        base: "IBase",
        declared_in: "AEEAClock.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IControl", ret: "boolean",
                params: &["IControl*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Redraw", slot: 3, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetActive", slot: 4, base: "IControl", ret: "void",
                params: &["IControl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "IsActive", slot: 5, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetRect", slot: 6, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRect", slot: 7, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "SetProperties", slot: 8, base: "IControl", ret: "void",
                params: &["IControl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 9, base: "IControl", ret: "uint32",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 10, base: "IControl", ret: "void",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Start", slot: 11, base: "IAClockCtl", ret: "void",
                params: &["IAClockCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "Stop", slot: 12, base: "IAClockCtl", ret: "void",
                params: &["IAClockCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetTime", slot: 13, base: "IAClockCtl", ret: "uint32",
                params: &["IAClockCtl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "SetFaceColor", slot: 14, base: "IAClockCtl", ret: "void",
                params: &["IAClockCtl*", "RGBVAL"],
                words: 2,
            },
            MethodInfo {
                name: "SetHandFrameColor", slot: 15, base: "IAClockCtl", ret: "void",
                params: &["IAClockCtl*", "RGBVAL"],
                words: 2,
            },
            MethodInfo {
                name: "SetSecColor", slot: 16, base: "IAClockCtl", ret: "void",
                params: &["IAClockCtl*", "RGBVAL"],
                words: 2,
            },
            MethodInfo {
                name: "SetMinColor", slot: 17, base: "IAClockCtl", ret: "void",
                params: &["IAClockCtl*", "RGBVAL"],
                words: 2,
            },
            MethodInfo {
                name: "SetHourColor", slot: 18, base: "IAClockCtl", ret: "void",
                params: &["IAClockCtl*", "RGBVAL"],
                words: 2,
            },
            MethodInfo {
                name: "SetFaceImage", slot: 19, base: "IAClockCtl", ret: "void",
                params: &["IAClockCtl*", "IImage*", "AEERect*"],
                words: 3,
            },
            MethodInfo {
                name: "GetFaceImageRect", slot: 20, base: "IAClockCtl", ret: "void",
                params: &["IAClockCtl*", "AEERect*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IAStream",
        base: "IBase",
        declared_in: "AEEIAStream.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Readable", slot: 2, base: "IAStream", ret: "void",
                params: &["IAStream*", "fnptr", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "IAStream", ret: "int32",
                params: &["IAStream*", "void*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Cancel", slot: 4, base: "IAStream", ret: "void",
                params: &["IAStream*", "fnptr", "void*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IAddrBook",
        base: "IBase",
        declared_in: "AEEAddrBook.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumCategoryInit", slot: 2, base: "IAddrBook", ret: "int",
                params: &["IAddrBook*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumNextCategory", slot: 3, base: "IAddrBook", ret: "boolean",
                params: &["IAddrBook*", "AEEAddrCat*"],
                words: 2,
            },
            MethodInfo {
                name: "EnumFieldsInfoInit", slot: 4, base: "IAddrBook", ret: "int",
                params: &["IAddrBook*", "AEEAddrCat"],
                words: 2,
            },
            MethodInfo {
                name: "EnumNextFieldsInfo", slot: 5, base: "IAddrBook", ret: "boolean",
                params: &["IAddrBook*", "AEEAddrFieldInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "CreateRec", slot: 6, base: "IAddrBook", ret: "IAddrRec*",
                params: &["IAddrBook*", "AEEAddrCat", "AEEAddrField*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetNumRecs", slot: 7, base: "IAddrBook", ret: "uint16",
                params: &["IAddrBook*"],
                words: 1,
            },
            MethodInfo {
                name: "RemoveAllRecs", slot: 8, base: "IAddrBook", ret: "int",
                params: &["IAddrBook*"],
                words: 1,
            },
            MethodInfo {
                name: "GetRecByID", slot: 9, base: "IAddrBook", ret: "IAddrRec*",
                params: &["IAddrBook*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "EnumRecInit", slot: 10, base: "IAddrBook", ret: "int",
                params: &["IAddrBook*", "AEEAddrCat", "AEEAddrFieldID", "void*", "uint16"],
                words: 5,
            },
            MethodInfo {
                name: "EnumNextRec", slot: 11, base: "IAddrBook", ret: "IAddrRec*",
                params: &["IAddrBook*"],
                words: 1,
            },
            MethodInfo {
                name: "GetLastError", slot: 12, base: "IAddrBook", ret: "int",
                params: &["IAddrBook*"],
                words: 1,
            },
            MethodInfo {
                name: "SetProperties", slot: 13, base: "IAddrBook", ret: "void",
                params: &["IAddrBook*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 14, base: "IAddrBook", ret: "uint32",
                params: &["IAddrBook*"],
                words: 1,
            },
            MethodInfo {
                name: "GetCategoryName", slot: 15, base: "IAddrBook", ret: "int",
                params: &["IAddrBook*", "AEEAddrCat", "AECHAR*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetFieldName", slot: 16, base: "IAddrBook", ret: "int",
                params: &["IAddrBook*", "AEEAddrFieldID", "AECHAR*", "int*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IAddrInfo",
        base: "IQI",
        declared_in: "AEEIAddrInfo.h",
        iid: 0x01036e17,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SelectNetwork", slot: 3, base: "IAddrInfo", ret: "int",
                params: &["IAddrInfo*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "StartQuery", slot: 4, base: "IAddrInfo", ret: "int",
                params: &["IAddrInfo*", "char*", "char*", "AEEAddrInfo*", "AEECallback*"],
                words: 5,
            },
            MethodInfo {
                name: "GetResult", slot: 5, base: "IAddrInfo", ret: "int",
                params: &["IAddrInfo*", "int*", "int*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetResultAddr", slot: 6, base: "IAddrInfo", ret: "int",
                params: &["IAddrInfo*", "int", "AEEAddrInfo*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "ExtFunc", slot: 7, base: "IAddrInfo", ret: "int",
                params: &["IAddrInfo*", "AEECLSID", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IAddrInfoCache",
        base: "IQI",
        declared_in: "AEEIAddrInfoCache.h",
        iid: 0x01037ae6,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SelectNetwork", slot: 3, base: "IAddrInfoCache", ret: "int",
                params: &["IAddrInfoCache*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "ClearCache", slot: 4, base: "IAddrInfoCache", ret: "int",
                params: &["IAddrInfoCache*"],
                words: 1,
            },
            MethodInfo {
                name: "ClearCacheEntry", slot: 5, base: "IAddrInfoCache", ret: "int",
                params: &["IAddrInfoCache*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "ExtFunc", slot: 6, base: "IAddrInfoCache", ret: "int",
                params: &["IAddrInfoCache*", "AEECLSID", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IAddrRec",
        base: "IBase",
        declared_in: "AEEAddrBook.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "GetCategory", slot: 2, base: "IAddrRec", ret: "AEEAddrCat",
                params: &["IAddrRec*"],
                words: 1,
            },
            MethodInfo {
                name: "SetCategory", slot: 3, base: "IAddrRec", ret: "int",
                params: &["IAddrRec*", "AEEAddrCat"],
                words: 2,
            },
            MethodInfo {
                name: "GetRecID", slot: 4, base: "IAddrRec", ret: "uint16",
                params: &["IAddrRec*"],
                words: 1,
            },
            MethodInfo {
                name: "GetFieldCount", slot: 5, base: "IAddrRec", ret: "int",
                params: &["IAddrRec*"],
                words: 1,
            },
            MethodInfo {
                name: "GetField", slot: 6, base: "IAddrRec", ret: "AEEAddrField*",
                params: &["IAddrRec*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "UpdateField", slot: 7, base: "IAddrRec", ret: "int",
                params: &["IAddrRec*", "int", "AEEAddrField*"],
                words: 3,
            },
            MethodInfo {
                name: "UpdateAllFields", slot: 8, base: "IAddrRec", ret: "int",
                params: &["IAddrRec*", "AEEAddrField*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "AddField", slot: 9, base: "IAddrRec", ret: "int",
                params: &["IAddrRec*", "AEEAddrField*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveField", slot: 10, base: "IAddrRec", ret: "int",
                params: &["IAddrRec*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveRec", slot: 11, base: "IAddrRec", ret: "int",
                params: &["IAddrRec*"],
                words: 1,
            },
            MethodInfo {
                name: "GetLastError", slot: 12, base: "IAddrRec", ret: "int",
                params: &["IAddrRec*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IAlarmMgr",
        base: "IQueryInterface",
        declared_in: "AEEAlarmMgr.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SetAlarm", slot: 3, base: "IAlarmMgr", ret: "int",
                params: &["IAlarmMgr*", "AEECLSID", "uint32*", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "CancelAlarm", slot: 4, base: "IAlarmMgr", ret: "int",
                params: &["IAlarmMgr*", "AEECLSID", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "AlarmsActive", slot: 5, base: "IAlarmMgr", ret: "int",
                params: &["IAlarmMgr*", "AEECLSID"],
                words: 2,
            },
            MethodInfo {
                name: "ListAlarmCodes", slot: 6, base: "IAlarmMgr", ret: "int",
                params: &["IAlarmMgr*", "AEECLSID", "uint16*", "uint32*"],
                words: 4,
            },
            MethodInfo {
                name: "ReserveUserCodes", slot: 7, base: "IAlarmMgr", ret: "int",
                params: &["IAlarmMgr*", "AEECLSID", "uint16*", "uint32*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IAppHistory",
        base: "IQI",
        declared_in: "AEEIAppHistory.h",
        iid: 0x010396bf,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Forward", slot: 3, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*"],
                words: 1,
            },
            MethodInfo {
                name: "Back", slot: 4, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*"],
                words: 1,
            },
            MethodInfo {
                name: "Top", slot: 5, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*"],
                words: 1,
            },
            MethodInfo {
                name: "Insert", slot: 6, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*", "AEECLSID", "char*"],
                words: 3,
            },
            MethodInfo {
                name: "GetClass", slot: 7, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*", "AEECLSID*"],
                words: 2,
            },
            MethodInfo {
                name: "GetArgs", slot: 8, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*", "char*", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "GetData", slot: 9, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*", "char*", "void*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "SetData", slot: 10, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*", "char*", "void*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "Current", slot: 11, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*", "int*"],
                words: 2,
            },
            MethodInfo {
                name: "Remove", slot: 12, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*"],
                words: 1,
            },
            MethodInfo {
                name: "Move", slot: 13, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "GetReason", slot: 14, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*", "uint32*"],
                words: 2,
            },
            MethodInfo {
                name: "SetReason", slot: 15, base: "IAppHistory", ret: "int",
                params: &["IAppHistory*", "uint32"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IApplet",
        base: "IBase",
        declared_in: "AEEIApplet.h",
        iid: 0x01010100,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IApplet", ret: "boolean",
                params: &["IApplet*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IAppletCtl",
        base: "IQI",
        declared_in: "AEEIAppletCtl.h",
        iid: 0x0103a4d8,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetRunningList", slot: 3, base: "IAppletCtl", ret: "int",
                params: &["IAppletCtl*", "void*", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "GetRunningInfo", slot: 4, base: "IAppletCtl", ret: "int",
                params: &["IAppletCtl*", "AEECLSID", "AEEAppItem", "void*", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "Control", slot: 5, base: "IAppletCtl", ret: "int",
                params: &["IAppletCtl*", "AEECLSID", "int", "void*", "int*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IBCMCSDB",
        base: "IQI",
        declared_in: "AEEIBCMCSDB.h",
        iid: 0x01036173,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Update", slot: 3, base: "IBCMCSDB", ret: "int",
                params: &["IBCMCSDB*", "IxOpts*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "ExtFunc", slot: 4, base: "IBCMCSDB", ret: "int",
                params: &["IBCMCSDB*", "AEECLSID", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IBN",
        base: "IQI",
        declared_in: "AEEIBN.h",
        iid: 0x0102eaed,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "New", slot: 3, base: "IBN", ret: "int",
                params: &["IBN*", "byte*", "int", "IBN**"],
                words: 4,
            },
            MethodInfo {
                name: "Set", slot: 4, base: "IBN", ret: "int",
                params: &["IBN*", "byte*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Copy", slot: 5, base: "IBN", ret: "int",
                params: &["IBN*", "IBN*"],
                words: 2,
            },
            MethodInfo {
                name: "Get", slot: 6, base: "IBN", ret: "int",
                params: &["IBN*", "byte*", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "ModExp", slot: 7, base: "IBN", ret: "void",
                params: &["IBN*", "IBN*", "IBN*", "int*", "AEECallback*"],
                words: 5,
            },
            MethodInfo {
                name: "ModMul", slot: 8, base: "IBN", ret: "int",
                params: &["IBN*", "IBN*", "IBN*"],
                words: 3,
            },
            MethodInfo {
                name: "Add", slot: 9, base: "IBN", ret: "int",
                params: &["IBN*", "IBN*"],
                words: 2,
            },
            MethodInfo {
                name: "Sub", slot: 10, base: "IBN", ret: "int",
                params: &["IBN*", "IBN*"],
                words: 2,
            },
            MethodInfo {
                name: "Mul", slot: 11, base: "IBN", ret: "int",
                params: &["IBN*", "IBN*"],
                words: 2,
            },
            MethodInfo {
                name: "Div", slot: 12, base: "IBN", ret: "int",
                params: &["IBN*", "IBN*", "IBN*"],
                words: 3,
            },
            MethodInfo {
                name: "Cmp", slot: 13, base: "IBN", ret: "int",
                params: &["IBN*", "IBN*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IBNEng",
        base: "IParameters1",
        declared_in: "AEEIBNEng.h",
        iid: 0x0103a3f1,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "SetParam", slot: 4, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetOperationContext", slot: 5, base: "IBNEng", ret: "int",
                params: &["IBNEng*", "IBN*", "IBN**"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IBT",
        base: "IQueryInterface",
        declared_in: "AEEBT.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Command", slot: 3, base: "IBT", ret: "int",
                params: &["IBT*", "int", "AEECallback*", "void*", "int", "void*", "int"],
                words: 7,
            },
            MethodInfo {
                name: "GetObject", slot: 4, base: "IBT", ret: "int",
                params: &["IBT*", "AEECLSID", "void*", "int", "void**"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IBTA2DP",
        base: "IQI",
        declared_in: "AEEIBTA2DP.h",
        iid: 0x01061c2b,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Enable", slot: 3, base: "IBTA2DP", ret: "int",
                params: &["IBTA2DP*", "BTSecurityLevel"],
                words: 2,
            },
            MethodInfo {
                name: "Disable", slot: 4, base: "IBTA2DP", ret: "int",
                params: &["IBTA2DP*"],
                words: 1,
            },
            MethodInfo {
                name: "SetDevice", slot: 5, base: "IBTA2DP", ret: "int",
                params: &["IBTA2DP*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "Connect", slot: 6, base: "IBTA2DP", ret: "int",
                params: &["IBTA2DP*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "Disconnect", slot: 7, base: "IBTA2DP", ret: "int",
                params: &["IBTA2DP*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "Configure", slot: 8, base: "IBTA2DP", ret: "int",
                params: &["IBTA2DP*", "BDAddress*", "BTA2DPConfig", "char*"],
                words: 4,
            },
            MethodInfo {
                name: "GetConfig", slot: 9, base: "IBTA2DP", ret: "int",
                params: &["IBTA2DP*", "BDAddress*", "char*", "int", "int*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IBTA2DPNotifier",
        base: "IQI",
        declared_in: "AEEIBTA2DPNotifier.h",
        iid: 0x01061c2a,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "OnEventNotify", slot: 3, base: "IBTA2DPNotifier", ret: "int",
                params: &["IBTA2DPNotifier*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetEvent", slot: 4, base: "IBTA2DPNotifier", ret: "int",
                params: &["IBTA2DPNotifier*", "BTA2DPEvent*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IBTAVRCPTarget",
        base: "IQI",
        declared_in: "AEEIBTAVRCPTarget.h",
        iid: 0x01062249,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Enable", slot: 3, base: "IBTAVRCPTarget", ret: "AEEResult",
                params: &["IBTAVRCPTarget*", "BTSecurityLevel"],
                words: 2,
            },
            MethodInfo {
                name: "Connect", slot: 4, base: "IBTAVRCPTarget", ret: "AEEResult",
                params: &["IBTAVRCPTarget*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "Disconnect", slot: 5, base: "IBTAVRCPTarget", ret: "AEEResult",
                params: &["IBTAVRCPTarget*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "Disable", slot: 6, base: "IBTAVRCPTarget", ret: "AEEResult",
                params: &["IBTAVRCPTarget*"],
                words: 1,
            },
            MethodInfo {
                name: "GetStatus", slot: 7, base: "IBTAVRCPTarget", ret: "AEEResult",
                params: &["IBTAVRCPTarget*", "BTSecurityLevel*", "boolean*", "BDAddress*", "int", "int*"],
                words: 6,
            },
            MethodInfo {
                name: "OnEventNotify", slot: 8, base: "IBTAVRCPTarget", ret: "AEEResult",
                params: &["IBTAVRCPTarget*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetEvent", slot: 9, base: "IBTAVRCPTarget", ret: "AEEResult",
                params: &["IBTAVRCPTarget*", "AEEBTAVRCPEventType*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IBTAudioGateway",
        base: "IQI",
        declared_in: "AEEIBTAudioGateway.h",
        iid: 0x0105aa38,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "OnOpResult", slot: 3, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetOpResult", slot: 4, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "BTAGOperationType*", "BTResult*"],
                words: 3,
            },
            MethodInfo {
                name: "OnEventNotify", slot: 5, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetEvent", slot: 6, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "BTAGEventType*"],
                words: 2,
            },
            MethodInfo {
                name: "GetConnectionList", slot: 7, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "BTAGConnection*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetStatus", slot: 8, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "BTAGStatus*"],
                words: 2,
            },
            MethodInfo {
                name: "Config", slot: 9, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "BTAGConfigFlags", "BTAGButtonFunction", "BTSecurityLevel", "char*", "BDAddress*", "int"],
                words: 7,
            },
            MethodInfo {
                name: "ConfigEx", slot: 10, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "Enable", slot: 11, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*"],
                words: 1,
            },
            MethodInfo {
                name: "Disable", slot: 12, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*"],
                words: 1,
            },
            MethodInfo {
                name: "Connect", slot: 13, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "Disconnect", slot: 14, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "ConnectAudio", slot: 15, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "DisconnectAudio", slot: 16, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "ReportVRState", slot: 17, base: "IBTAudioGateway", ret: "int",
                params: &["IBTAudioGateway*", "boolean"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IBTConnection",
        base: "IQI",
        declared_in: "AEEIBTConnection.h",
        iid: 0x0105a9ad,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "OnEventNotify", slot: 3, base: "IBTConnection", ret: "int",
                params: &["IBTConnection*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetEvent", slot: 4, base: "IBTConnection", ret: "int",
                params: &["IBTConnection*", "BTConnEventType*", "BTResult*"],
                words: 3,
            },
            MethodInfo {
                name: "GetStatus", slot: 5, base: "IBTConnection", ret: "int",
                params: &["IBTConnection*", "BTConnStatBitmap*", "BTConnectionStatus*"],
                words: 3,
            },
            MethodInfo {
                name: "SetRoleSwitchPolicy", slot: 6, base: "IBTConnection", ret: "int",
                params: &["IBTConnection*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "SetRole", slot: 7, base: "IBTConnection", ret: "int",
                params: &["IBTConnection*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "SetMinSecurity", slot: 8, base: "IBTConnection", ret: "int",
                params: &["IBTConnection*", "BTSecurityLevel"],
                words: 2,
            },
            MethodInfo {
                name: "SetLowPowerConfig", slot: 9, base: "IBTConnection", ret: "int",
                params: &["IBTConnection*", "BTLowPowerMode", "uint16", "uint16"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IBTDevDB",
        base: "IQueryInterface",
        declared_in: "AEEBTDevDB.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Add", slot: 3, base: "IBTDevDB", ret: "BTResult",
                params: &["IBTDevDB*", "BTDeviceInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "Read", slot: 4, base: "IBTDevDB", ret: "BTResult",
                params: &["IBTDevDB*", "BTDBEntry*"],
                words: 2,
            },
            MethodInfo {
                name: "Update", slot: 5, base: "IBTDevDB", ret: "BTResult",
                params: &["IBTDevDB*", "BTModDevField", "BTDBEntry*"],
                words: 3,
            },
            MethodInfo {
                name: "EnumInit", slot: 6, base: "IBTDevDB", ret: "BTResult",
                params: &["IBTDevDB*", "BTDevDBEnumerator*"],
                words: 2,
            },
            MethodInfo {
                name: "EnumNext", slot: 7, base: "IBTDevDB", ret: "BTResult",
                params: &["IBTDevDB*", "BTDBEntry*"],
                words: 2,
            },
            MethodInfo {
                name: "Bond", slot: 8, base: "IBTDevDB", ret: "int",
                params: &["IBTDevDB*", "BDAddress*", "uint8*", "boolean", "BTResult*", "AEECallback*"],
                words: 6,
            },
        ],
    },
    IfaceInfo {
        name: "IBTMgr",
        base: "IQI",
        declared_in: "AEEIBTMgr.h",
        iid: 0x0105397c,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "OnEventNotify", slot: 3, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetEvent", slot: 4, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "BTMgrEventType*", "BTResult*"],
                words: 3,
            },
            MethodInfo {
                name: "GetRadioState", slot: 5, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "BTRadioState*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLocalSettings", slot: 6, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "BTLocalSettingBitmap*", "BTLocalSettings*"],
                words: 3,
            },
            MethodInfo {
                name: "SetName", slot: 7, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "uint8*"],
                words: 2,
            },
            MethodInfo {
                name: "SetCoD", slot: 8, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "BTClassOfDevice*"],
                words: 2,
            },
            MethodInfo {
                name: "SetSecurity", slot: 9, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "BTSecurityLevel"],
                words: 2,
            },
            MethodInfo {
                name: "SetPrefConnRole", slot: 10, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "SetRoleSwitchPolicy", slot: 11, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "SetBlockedDevList", slot: 12, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "int", "BDAddress*"],
                words: 3,
            },
            MethodInfo {
                name: "SetSvcSecurity", slot: 13, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "BTSecurityLevel", "BTServiceIdentifier*"],
                words: 3,
            },
            MethodInfo {
                name: "OnConnRequest", slot: 14, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetConnRequest", slot: 15, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "BDAddress*", "BTServiceIdentifier*"],
                words: 3,
            },
            MethodInfo {
                name: "AuthorizeConnReq", slot: 16, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "BDAddress*", "BTServiceIdentifier*", "boolean"],
                words: 4,
            },
            MethodInfo {
                name: "TuneScanPerformance", slot: 17, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*", "BTActVsPwr", "BTActVsPwr"],
                words: 3,
            },
            MethodInfo {
                name: "DisableRadio", slot: 18, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*"],
                words: 1,
            },
            MethodInfo {
                name: "EnableRadio", slot: 19, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*"],
                words: 1,
            },
            MethodInfo {
                name: "DisableVisibility", slot: 20, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*"],
                words: 1,
            },
            MethodInfo {
                name: "RestoreVisibility", slot: 21, base: "IBTMgr", ret: "int",
                params: &["IBTMgr*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IBTSPP",
        base: "IPort",
        declared_in: "AEEBTSPP.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLastError", slot: 5, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Write", slot: 6, base: "IPort", ret: "int32",
                params: &["IPort*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Writeable", slot: 7, base: "IPort", ret: "void",
                params: &["IPort*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "IOCtl", slot: 8, base: "IPort", ret: "int",
                params: &["IPort*", "int", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Close", slot: 9, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Open", slot: 10, base: "IPort", ret: "int",
                params: &["IPort*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "OnStatusChange", slot: 11, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "OnModemStatusChange", slot: 12, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "OnLineError", slot: 13, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "Disconnect", slot: 14, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTResult*", "AEECallback*"],
                words: 3,
            },
            MethodInfo {
                name: "GetStatus", slot: 15, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPStatus*"],
                words: 2,
            },
            MethodInfo {
                name: "GetModemStatus", slot: 16, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPModemStatus*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLineError", slot: 17, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPLineError*"],
                words: 2,
            },
            MethodInfo {
                name: "Configure", slot: 18, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPConfig*", "BTResult*", "AEECallback*"],
                words: 4,
            },
            MethodInfo {
                name: "IOCtlEx", slot: 19, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPIOCtlCommand", "BTSPPModemStatus*", "AEECallback*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IBTSPPClient",
        base: "IBTSPP",
        declared_in: "AEEBTSPPClient.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLastError", slot: 5, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Write", slot: 6, base: "IPort", ret: "int32",
                params: &["IPort*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Writeable", slot: 7, base: "IPort", ret: "void",
                params: &["IPort*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "IOCtl", slot: 8, base: "IPort", ret: "int",
                params: &["IPort*", "int", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Close", slot: 9, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Open", slot: 10, base: "IPort", ret: "int",
                params: &["IPort*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "OnStatusChange", slot: 11, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "OnModemStatusChange", slot: 12, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "OnLineError", slot: 13, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "Disconnect", slot: 14, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTResult*", "AEECallback*"],
                words: 3,
            },
            MethodInfo {
                name: "GetStatus", slot: 15, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPStatus*"],
                words: 2,
            },
            MethodInfo {
                name: "GetModemStatus", slot: 16, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPModemStatus*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLineError", slot: 17, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPLineError*"],
                words: 2,
            },
            MethodInfo {
                name: "Configure", slot: 18, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPConfig*", "BTResult*", "AEECallback*"],
                words: 4,
            },
            MethodInfo {
                name: "IOCtlEx", slot: 19, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPIOCtlCommand", "BTSPPModemStatus*", "AEECallback*"],
                words: 4,
            },
            MethodInfo {
                name: "FindServersInit", slot: 20, base: "IBTSPPClient", ret: "int",
                params: &["IBTSPPClient*", "uint32", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "FindServersNext", slot: 21, base: "IBTSPPClient", ret: "int",
                params: &["IBTSPPClient*", "uint32*", "AEECallback*"],
                words: 3,
            },
            MethodInfo {
                name: "GetServerInfo", slot: 22, base: "IBTSPPClient", ret: "int",
                params: &["IBTSPPClient*", "BTSPPDeviceInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "Connect", slot: 23, base: "IBTSPPClient", ret: "int",
                params: &["IBTSPPClient*", "BDAddress*", "uint32", "BTResult*", "AEECallback*"],
                words: 5,
            },
            MethodInfo {
                name: "CancelServerSearch", slot: 24, base: "IBTSPPClient", ret: "int",
                params: &["IBTSPPClient*", "AEECallback*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IBTSPPServer",
        base: "IBTSPP",
        declared_in: "AEEBTSPPServer.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLastError", slot: 5, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Write", slot: 6, base: "IPort", ret: "int32",
                params: &["IPort*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Writeable", slot: 7, base: "IPort", ret: "void",
                params: &["IPort*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "IOCtl", slot: 8, base: "IPort", ret: "int",
                params: &["IPort*", "int", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Close", slot: 9, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Open", slot: 10, base: "IPort", ret: "int",
                params: &["IPort*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "OnStatusChange", slot: 11, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "OnModemStatusChange", slot: 12, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "OnLineError", slot: 13, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "Disconnect", slot: 14, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTResult*", "AEECallback*"],
                words: 3,
            },
            MethodInfo {
                name: "GetStatus", slot: 15, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPStatus*"],
                words: 2,
            },
            MethodInfo {
                name: "GetModemStatus", slot: 16, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPModemStatus*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLineError", slot: 17, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPLineError*"],
                words: 2,
            },
            MethodInfo {
                name: "Configure", slot: 18, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPConfig*", "BTResult*", "AEECallback*"],
                words: 4,
            },
            MethodInfo {
                name: "IOCtlEx", slot: 19, base: "IBTSPP", ret: "int",
                params: &["IBTSPP*", "BTSPPIOCtlCommand", "BTSPPModemStatus*", "AEECallback*"],
                words: 4,
            },
            MethodInfo {
                name: "EnableService", slot: 20, base: "IBTSPPServer", ret: "int",
                params: &["IBTSPPServer*", "BTSecurityLevel", "uint32", "boolean", "BTResult*", "AEECallback*"],
                words: 6,
            },
            MethodInfo {
                name: "DisableService", slot: 21, base: "IBTSPPServer", ret: "int",
                params: &["IBTSPPServer*", "BTResult*", "AEECallback*"],
                words: 3,
            },
            MethodInfo {
                name: "OnConnRequest", slot: 22, base: "IBTSPPServer", ret: "int",
                params: &["IBTSPPServer*", "AEECallback*", "BTEventData*"],
                words: 3,
            },
            MethodInfo {
                name: "Authorize", slot: 23, base: "IBTSPPServer", ret: "int",
                params: &["IBTSPPServer*", "BDAddress*", "boolean", "BTResult*", "AEECallback*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IBTServiceDiscovery",
        base: "IQI",
        declared_in: "AEEIBTServiceDiscovery.h",
        iid: 0x010570e0,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "OnEventNotify", slot: 3, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetEvent", slot: 4, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BTSDEventType*", "BTResult*"],
                words: 3,
            },
            MethodInfo {
                name: "StartDeviceSearch", slot: 5, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BTClassOfService", "int", "int"],
                words: 4,
            },
            MethodInfo {
                name: "StopDeviceSearch", slot: 6, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*"],
                words: 1,
            },
            MethodInfo {
                name: "GetNextResponse", slot: 7, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BTDeviceInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "StartNameRequest", slot: 8, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "StopNameRequest", slot: 9, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BDAddress*"],
                words: 2,
            },
            MethodInfo {
                name: "GetName", slot: 10, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BTDeviceInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "RegisterService", slot: 11, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "char*", "boolean"],
                words: 3,
            },
            MethodInfo {
                name: "GetMyRecordHandles", slot: 12, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "uint32*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "DeregisterService", slot: 13, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetLocalDBSummary", slot: 14, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BTServiceRecordSummary*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "ReadRecord", slot: 15, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "uint32", "char*", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "WriteRecord", slot: 16, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "uint32", "char*"],
                words: 3,
            },
            MethodInfo {
                name: "GetDiscoverableMode", slot: 17, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BTSvcDiscoverableMode*"],
                words: 2,
            },
            MethodInfo {
                name: "SetDiscoverableMode", slot: 18, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BTSvcDiscoverableMode"],
                words: 2,
            },
            MethodInfo {
                name: "StartServiceSearch", slot: 19, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "BDAddress*", "BTSvcSearchPattern*"],
                words: 3,
            },
            MethodInfo {
                name: "StopServiceSearch", slot: 20, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*"],
                words: 1,
            },
            MethodInfo {
                name: "GetNumFoundRecs", slot: 21, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "int*"],
                words: 2,
            },
            MethodInfo {
                name: "GetSearchResult", slot: 22, base: "IBTServiceDiscovery", ret: "int",
                params: &["IBTServiceDiscovery*", "char*", "int", "int*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IBacklight",
        base: "IQueryInterface",
        declared_in: "AEEBacklight.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Enable", slot: 3, base: "IBacklight", ret: "int",
                params: &["IBacklight*"],
                words: 1,
            },
            MethodInfo {
                name: "Disable", slot: 4, base: "IBacklight", ret: "int",
                params: &["IBacklight*"],
                words: 1,
            },
            MethodInfo {
                name: "IsEnabled", slot: 5, base: "IBacklight", ret: "boolean",
                params: &["IBacklight*"],
                words: 1,
            },
            MethodInfo {
                name: "GetBacklightInfo", slot: 6, base: "IBacklight", ret: "int",
                params: &["IBacklight*", "AEEBacklightInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "GetBrightnessLevel", slot: 7, base: "IBacklight", ret: "int",
                params: &["IBacklight*", "uint32*"],
                words: 2,
            },
            MethodInfo {
                name: "SetBrightnessLevel", slot: 8, base: "IBacklight", ret: "int",
                params: &["IBacklight*", "uint32"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IBase",
        base: "",
        declared_in: "AEEIBase.h",
        iid: 0x01036a7b,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IBattery",
        base: "IQueryInterface",
        declared_in: "AEEBattery.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetBatteryItem", slot: 3, base: "IBattery", ret: "int",
                params: &["IBattery*", "uint32", "uint32*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IBearerTechnology",
        base: "IxOpts",
        declared_in: "AEEIBearerTechnology.h",
        iid: 0x01046312,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "ExtFunc", slot: 6, base: "IBearerTechnology", ret: "int",
                params: &["IBearerTechnology*", "AEECLSID", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IBitFontFoundry",
        base: "",
        declared_in: "AEEBitFont.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBitFontFoundry", ret: "uint32",
                params: &["IBitFontFoundry*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBitFontFoundry", ret: "uint32",
                params: &["IBitFontFoundry*"],
                words: 1,
            },
            MethodInfo {
                name: "CreateFromBBF", slot: 2, base: "IBitFontFoundry", ret: "int",
                params: &["IBitFontFoundry*", "byte*", "int", "void*", "IFont**"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IBitmap",
        base: "IQI",
        declared_in: "AEEIBitmap.h",
        iid: 0x01001021,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "RGBToNative", slot: 3, base: "IBitmap", ret: "NativeColor",
                params: &["IBitmap*", "RGBVAL"],
                words: 2,
            },
            MethodInfo {
                name: "NativeToRGB", slot: 4, base: "IBitmap", ret: "RGBVAL",
                params: &["IBitmap*", "NativeColor"],
                words: 2,
            },
            MethodInfo {
                name: "DrawPixel", slot: 5, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "unsigned", "unsigned", "NativeColor", "AEERasterOp"],
                words: 5,
            },
            MethodInfo {
                name: "GetPixel", slot: 6, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "unsigned", "unsigned", "NativeColor*"],
                words: 4,
            },
            MethodInfo {
                name: "SetPixels", slot: 7, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "unsigned", "AEEPoint*", "NativeColor", "AEERasterOp"],
                words: 5,
            },
            MethodInfo {
                name: "DrawHScanline", slot: 8, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "unsigned", "unsigned", "unsigned", "NativeColor", "AEERasterOp"],
                words: 6,
            },
            MethodInfo {
                name: "FillRect", slot: 9, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "AEERect*", "NativeColor", "AEERasterOp"],
                words: 4,
            },
            MethodInfo {
                name: "BltIn", slot: 10, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "int", "int", "int", "int", "IBitmap*", "int", "int", "AEERasterOp"],
                words: 9,
            },
            MethodInfo {
                name: "BltOut", slot: 11, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "int", "int", "int", "int", "IBitmap*", "int", "int", "AEERasterOp"],
                words: 9,
            },
            MethodInfo {
                name: "GetInfo", slot: 12, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "AEEBitmapInfo*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "CreateCompatibleBitmap", slot: 13, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "IBitmap**", "uint16", "uint16"],
                words: 4,
            },
            MethodInfo {
                name: "SetTransparencyColor", slot: 14, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "NativeColor"],
                words: 2,
            },
            MethodInfo {
                name: "GetTransparencyColor", slot: 15, base: "IBitmap", ret: "int",
                params: &["IBitmap*", "NativeColor*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IBitmapDev",
        base: "IQI",
        declared_in: "AEEIBitmapDev.h",
        iid: 0x01012719,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Update", slot: 3, base: "IBitmapDev", ret: "int",
                params: &["IBitmapDev*"],
                words: 1,
            },
            MethodInfo {
                name: "IsEnabled", slot: 4, base: "IBitmapDev", ret: "boolean",
                params: &["IBitmapDev*"],
                words: 1,
            },
            MethodInfo {
                name: "NotifyEnable", slot: 5, base: "IBitmapDev", ret: "int",
                params: &["IBitmapDev*", "AEECallback*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IBitmapDev2",
        base: "IBitmapDev",
        declared_in: "AEEIBitmapDev2.h",
        iid: 0x0103f099,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Update", slot: 3, base: "IBitmapDev", ret: "int",
                params: &["IBitmapDev*"],
                words: 1,
            },
            MethodInfo {
                name: "IsEnabled", slot: 4, base: "IBitmapDev", ret: "boolean",
                params: &["IBitmapDev*"],
                words: 1,
            },
            MethodInfo {
                name: "NotifyEnable", slot: 5, base: "IBitmapDev", ret: "int",
                params: &["IBitmapDev*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "SetPrefs", slot: 6, base: "IBitmapDev2", ret: "int",
                params: &["IBitmapDev2*", "char*", "int"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IBitmapRot",
        base: "IQueryInterface",
        declared_in: "AEEBitmapRot.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetRotation", slot: 3, base: "IBitmapRot", ret: "int",
                params: &["IBitmapRot*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IBitmapScale",
        base: "IQI",
        declared_in: "AEEIBitmapScale.h",
        iid: 0x0101e8bc,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Scale", slot: 3, base: "IBitmapScale", ret: "int",
                params: &["IBitmapScale*", "IBitmap*", "AEERect*", "IBitmap*", "AEERect*", "AEERect*", "int"],
                words: 7,
            },
        ],
    },
    IfaceInfo {
        name: "ICache",
        base: "IRecordStore",
        declared_in: "AEERamCache.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Add", slot: 3, base: "IRecordStore", ret: "int",
                params: &["IRecordStore*", "char*", "int", "char*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "Remove", slot: 4, base: "IRecordStore", ret: "int",
                params: &["IRecordStore*", "char*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Find", slot: 5, base: "IRecordStore", ret: "int",
                params: &["IRecordStore*", "char*", "int", "char*", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "SetTTL", slot: 6, base: "ICache", ret: "uint32",
                params: &["ICache*", "uint32"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ICall",
        base: "IQueryInterface",
        declared_in: "AEETelephone.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetInfo", slot: 3, base: "ICall", ret: "int",
                params: &["ICall*", "AEETCallInfo*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "GetType", slot: 4, base: "ICall", ret: "AEETCallType",
                params: &["ICall*"],
                words: 1,
            },
            MethodInfo {
                name: "GetState", slot: 5, base: "ICall", ret: "AEETCallState",
                params: &["ICall*"],
                words: 1,
            },
            MethodInfo {
                name: "Answer", slot: 6, base: "ICall", ret: "int",
                params: &["ICall*"],
                words: 1,
            },
            MethodInfo {
                name: "End", slot: 7, base: "ICall", ret: "int",
                params: &["ICall*"],
                words: 1,
            },
            MethodInfo {
                name: "Redirect", slot: 8, base: "ICall", ret: "int",
                params: &["ICall*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "Transfer", slot: 9, base: "ICall", ret: "int",
                params: &["ICall*", "ICall*"],
                words: 2,
            },
            MethodInfo {
                name: "Hold", slot: 10, base: "ICall", ret: "int",
                params: &["ICall*"],
                words: 1,
            },
            MethodInfo {
                name: "GetCallDescriptor", slot: 11, base: "ICall", ret: "int",
                params: &["ICall*", "AEECallDesc*"],
                words: 2,
            },
            MethodInfo {
                name: "BurstDTMF", slot: 12, base: "ICall", ret: "int",
                params: &["ICall*", "AEETDTMFOnLength", "AEETDTMFOffLength", "char*"],
                words: 4,
            },
            MethodInfo {
                name: "StartDTMF", slot: 13, base: "ICall", ret: "int",
                params: &["ICall*", "char"],
                words: 2,
            },
            MethodInfo {
                name: "StopDTMF", slot: 14, base: "ICall", ret: "int",
                params: &["ICall*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "ICallHistory",
        base: "IQueryInterface",
        declared_in: "AEECallHistory.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddEntry", slot: 3, base: "ICallHistory", ret: "int",
                params: &["ICallHistory*", "AEECallHistoryEntry*"],
                words: 2,
            },
            MethodInfo {
                name: "EnumInit", slot: 4, base: "ICallHistory", ret: "int",
                params: &["ICallHistory*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumNext", slot: 5, base: "ICallHistory", ret: "AEECallHistoryEntry*",
                params: &["ICallHistory*", "int*"],
                words: 2,
            },
            MethodInfo {
                name: "UpdateEntry", slot: 6, base: "ICallHistory", ret: "int",
                params: &["ICallHistory*", "AEECallHistoryEntry*"],
                words: 2,
            },
            MethodInfo {
                name: "Clear", slot: 7, base: "ICallHistory", ret: "int",
                params: &["ICallHistory*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumInitByCallType", slot: 8, base: "ICallHistory", ret: "int",
                params: &["ICallHistory*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "ClearEntry", slot: 9, base: "ICallHistory", ret: "int",
                params: &["ICallHistory*"],
                words: 1,
            },
            MethodInfo {
                name: "Notify", slot: 10, base: "ICallHistory", ret: "int",
                params: &["ICallHistory*", "AEECallback*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ICallMgr",
        base: "IWebOpts",
        declared_in: "AEETelephone.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "Originate", slot: 6, base: "ICallMgr", ret: "int",
                params: &["ICallMgr*", "AEETCallType", "char*", "AECHAR*", "ICall**", "CallListener*"],
                words: 6,
            },
            MethodInfo {
                name: "OriginateEx", slot: 7, base: "ICallMgr", ret: "int",
                params: &["ICallMgr*", "AEETCallType", "ICall**", "CallListener*", "AEETOriginateOpt*"],
                words: 5,
            },
            MethodInfo {
                name: "GetCall", slot: 8, base: "ICallMgr", ret: "int",
                params: &["ICallMgr*", "AEECallDesc", "ICall**"],
                words: 3,
            },
            MethodInfo {
                name: "GetActiveVoiceCall", slot: 9, base: "ICallMgr", ret: "int",
                params: &["ICallMgr*", "ICall**"],
                words: 2,
            },
            MethodInfo {
                name: "EndAllCalls", slot: 10, base: "ICallMgr", ret: "int",
                params: &["ICallMgr*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "ICamera",
        base: "IQueryInterface",
        declared_in: "AEECamera.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "RegisterNotify", slot: 3, base: "ICamera", ret: "int",
                params: &["ICamera*", "PFNCAMERANOTIFY", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "SetParm", slot: 4, base: "ICamera", ret: "int",
                params: &["ICamera*", "int16", "int32", "int32"],
                words: 4,
            },
            MethodInfo {
                name: "GetParm", slot: 5, base: "ICamera", ret: "int",
                params: &["ICamera*", "int16", "int32*", "int32*"],
                words: 4,
            },
            MethodInfo {
                name: "Start", slot: 6, base: "ICamera", ret: "int",
                params: &["ICamera*", "int16", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Stop", slot: 7, base: "ICamera", ret: "int",
                params: &["ICamera*"],
                words: 1,
            },
            MethodInfo {
                name: "Pause", slot: 8, base: "ICamera", ret: "int",
                params: &["ICamera*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "GetFrame", slot: 9, base: "ICamera", ret: "int",
                params: &["ICamera*", "IBitmap**"],
                words: 2,
            },
            MethodInfo {
                name: "EncodeSnapshot", slot: 10, base: "ICamera", ret: "int",
                params: &["ICamera*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "ICertBag",
        base: "IxOpts",
        declared_in: "AEEICertBag.h",
        iid: 0x010386d5,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "AddCert", slot: 6, base: "ICertBag", ret: "int",
                params: &["ICertBag*", "int32", "uint8*", "int", "int*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "ICertChain",
        base: "ICertBag",
        declared_in: "AEEICertChain.h",
        iid: 0x0102eae4,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "AddCert", slot: 6, base: "ICertBag", ret: "int",
                params: &["ICertBag*", "int32", "uint8*", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "VerifyV", slot: 7, base: "ICertChain", ret: "void",
                params: &["ICertChain*", "CertDataTrustResult*", "AEECallback*", "xOpt*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "ICertParse",
        base: "IParameters1",
        declared_in: "AEEICertParse.h",
        iid: 0x0103937c,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "SetParam", slot: 4, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "SetCertByOpt", slot: 5, base: "ICertParse", ret: "int",
                params: &["ICertParse*", "IxOpts*", "int", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetStruct", slot: 6, base: "ICertParse", ret: "int",
                params: &["ICertParse*", "int", "void*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetField", slot: 7, base: "ICertParse", ret: "int",
                params: &["ICertParse*", "int", "uint8**", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetFieldPart", slot: 8, base: "ICertParse", ret: "int",
                params: &["ICertParse*", "CertDataPartRequest*", "CertDataCertPart*"],
                words: 3,
            },
            MethodInfo {
                name: "GetPubKeyParams", slot: 9, base: "ICertParse", ret: "int",
                params: &["ICertParse*", "IParameters1*"],
                words: 2,
            },
            MethodInfo {
                name: "Find", slot: 10, base: "ICertParse", ret: "int",
                params: &["ICertParse*", "IxOpts*", "CertDataIssuerAndSerial*", "int32*", "int32*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "ICharsetConv",
        base: "IQI",
        declared_in: "AEEICharsetConv.h",
        iid: 0x01044104,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Initialize", slot: 3, base: "ICharsetConv", ret: "int",
                params: &["ICharsetConv*", "char*", "char*", "unsigned int"],
                words: 4,
            },
            MethodInfo {
                name: "Convert", slot: 4, base: "ICharsetConv", ret: "int",
                params: &["ICharsetConv*", "unsigned char**", "int*", "unsigned char**", "int*", "int*"],
                words: 6,
            },
        ],
    },
    IfaceInfo {
        name: "ICipher",
        base: "IBase",
        declared_in: "AEESecurity.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Init", slot: 2, base: "ICipher", ret: "int",
                params: &["ICipher*", "byte*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Cipher", slot: 3, base: "ICipher", ret: "int",
                params: &["ICipher*", "byte*", "int", "byte*", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "Flush", slot: 4, base: "ICipher", ret: "void",
                params: &["ICipher*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "ICipher1",
        base: "IParameters",
        declared_in: "AEEICipher1.h",
        iid: 0x0102cce3,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParametersRO", ret: "int",
                params: &["IParametersRO*", "int", "void*", "unsigned*"],
                words: 4,
            },
            MethodInfo {
                name: "SetParam", slot: 4, base: "IParameters", ret: "int",
                params: &["IParameters*", "int", "void*", "unsigned"],
                words: 4,
            },
            MethodInfo {
                name: "Process", slot: 5, base: "ICipher1", ret: "int",
                params: &["ICipher1*", "byte*", "unsigned", "byte*", "unsigned*"],
                words: 5,
            },
            MethodInfo {
                name: "ProcessLast", slot: 6, base: "ICipher1", ret: "int",
                params: &["ICipher1*", "byte*", "unsigned*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "ICipherFactory",
        base: "IQI",
        declared_in: "AEEICipherFactory.h",
        iid: 0x0104171d,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "CreateCipher", slot: 3, base: "ICipherFactory", ret: "int",
                params: &["ICipherFactory*", "AEECLSID", "int", "AEECLSID", "int", "ICipher1**"],
                words: 6,
            },
            MethodInfo {
                name: "CreateCipher2", slot: 4, base: "ICipherFactory", ret: "int",
                params: &["ICipherFactory*", "CipherInfo*", "unsigned", "ICipher1**"],
                words: 4,
            },
            MethodInfo {
                name: "QueryCipher", slot: 5, base: "ICipherFactory", ret: "int",
                params: &["ICipherFactory*", "AEECLSID", "AEECLSID", "int", "unsigned"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "ICipherWrapper",
        base: "ICipher1",
        declared_in: "AEEICipherWrapper.h",
        iid: 0x0102ccdf,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParametersRO", ret: "int",
                params: &["IParametersRO*", "int", "void*", "unsigned*"],
                words: 4,
            },
            MethodInfo {
                name: "SetParam", slot: 4, base: "IParameters", ret: "int",
                params: &["IParameters*", "int", "void*", "unsigned"],
                words: 4,
            },
            MethodInfo {
                name: "Process", slot: 5, base: "ICipher1", ret: "int",
                params: &["ICipher1*", "byte*", "unsigned", "byte*", "unsigned*"],
                words: 5,
            },
            MethodInfo {
                name: "ProcessLast", slot: 6, base: "ICipher1", ret: "int",
                params: &["ICipher1*", "byte*", "unsigned*"],
                words: 3,
            },
            MethodInfo {
                name: "SetCipher", slot: 7, base: "ICipherWrapper", ret: "int",
                params: &["ICipherWrapper*", "IQI*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IClipboard",
        base: "IBase",
        declared_in: "AEEIClipboard.h",
        iid: 0x010367a8,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Empty", slot: 2, base: "IClipboard", ret: "int",
                params: &["IClipboard*"],
                words: 1,
            },
            MethodInfo {
                name: "GetSequence", slot: 3, base: "IClipboard", ret: "uint32",
                params: &["IClipboard*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumInit", slot: 4, base: "IClipboard", ret: "boolean",
                params: &["IClipboard*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumNext", slot: 5, base: "IClipboard", ret: "boolean",
                params: &["IClipboard*", "CBItemInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "GetData", slot: 6, base: "IClipboard", ret: "int",
                params: &["IClipboard*", "char*", "void*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "SetData", slot: 7, base: "IClipboard", ret: "int",
                params: &["IClipboard*", "char*", "void*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "MaxFreeBlock", slot: 8, base: "IClipboard", ret: "uint32",
                params: &["IClipboard*", "uint32*", "uint32*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IControl",
        base: "IBase",
        declared_in: "AEE.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IControl", ret: "boolean",
                params: &["IControl*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Redraw", slot: 3, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetActive", slot: 4, base: "IControl", ret: "void",
                params: &["IControl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "IsActive", slot: 5, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetRect", slot: 6, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRect", slot: 7, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "SetProperties", slot: 8, base: "IControl", ret: "void",
                params: &["IControl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 9, base: "IControl", ret: "uint32",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 10, base: "IControl", ret: "void",
                params: &["IControl*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IDBMgr",
        base: "IBase",
        declared_in: "AEEDB.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "OpenDatabase", slot: 2, base: "IDBMgr", ret: "IDatabase*",
                params: &["IDBMgr*", "char*", "boolean", "uint32", "uint16"],
                words: 5,
            },
            MethodInfo {
                name: "Remove", slot: 3, base: "IDBMgr", ret: "int",
                params: &["IDBMgr*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "SetCacheSize", slot: 4, base: "IDBMgr", ret: "void",
                params: &["IDBMgr*", "int"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IDBRecord",
        base: "IBase",
        declared_in: "AEEDB.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 2, base: "IDBRecord", ret: "void",
                params: &["IDBRecord*"],
                words: 1,
            },
            MethodInfo {
                name: "Update", slot: 3, base: "IDBRecord", ret: "int",
                params: &["IDBRecord*", "AEEDBField*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Remove", slot: 4, base: "IDBRecord", ret: "int",
                params: &["IDBRecord*"],
                words: 1,
            },
            MethodInfo {
                name: "GetID", slot: 5, base: "IDBRecord", ret: "uint16",
                params: &["IDBRecord*"],
                words: 1,
            },
            MethodInfo {
                name: "NextField", slot: 6, base: "IDBRecord", ret: "AEEDBFieldType",
                params: &["IDBRecord*", "AEEDBFieldName*", "uint16*"],
                words: 3,
            },
            MethodInfo {
                name: "GetField", slot: 7, base: "IDBRecord", ret: "byte*",
                params: &["IDBRecord*", "AEEDBFieldName*", "AEEDBFieldType*", "uint16*"],
                words: 4,
            },
            MethodInfo {
                name: "GetFieldWord", slot: 8, base: "IDBRecord", ret: "boolean",
                params: &["IDBRecord*", "uint16*"],
                words: 2,
            },
            MethodInfo {
                name: "GetFieldDWord", slot: 9, base: "IDBRecord", ret: "boolean",
                params: &["IDBRecord*", "uint32*"],
                words: 2,
            },
            MethodInfo {
                name: "GetFieldString", slot: 10, base: "IDBRecord", ret: "AECHAR*",
                params: &["IDBRecord*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IDLS",
        base: "IQueryInterface",
        declared_in: "AEEMediaDLS.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Load", slot: 3, base: "IDLS", ret: "int",
                params: &["IDLS*", "AEEMediaData*", "AEECallback*", "int32*"],
                words: 4,
            },
            MethodInfo {
                name: "Unload", slot: 4, base: "IDLS", ret: "int",
                params: &["IDLS*", "AEECallback*", "int32*"],
                words: 3,
            },
            MethodInfo {
                name: "IsLoaded", slot: 5, base: "IDLS", ret: "int",
                params: &["IDLS*", "boolean*"],
                words: 2,
            },
            MethodInfo {
                name: "IsGlobal", slot: 6, base: "IDLS", ret: "int",
                params: &["IDLS*", "boolean*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IDLSLinker",
        base: "IQueryInterface",
        declared_in: "AEEMediaDLS.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Link", slot: 3, base: "IDLSLinker", ret: "int",
                params: &["IDLSLinker*", "IDLS*"],
                words: 2,
            },
            MethodInfo {
                name: "Unlink", slot: 4, base: "IDLSLinker", ret: "int",
                params: &["IDLSLinker*", "IDLS*"],
                words: 2,
            },
            MethodInfo {
                name: "GetDLSList", slot: 5, base: "IDLSLinker", ret: "int",
                params: &["IDLSLinker*", "IDLS**", "int*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IDNS",
        base: "IQI",
        declared_in: "AEEIDNS.h",
        iid: 0x01005006,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddQuestion", slot: 3, base: "IDNS", ret: "int",
                params: &["IDNS*", "AEEDNSQType", "AEEDNSQClass", "char*"],
                words: 4,
            },
            MethodInfo {
                name: "Start", slot: 4, base: "IDNS", ret: "int",
                params: &["IDNS*", "CallbackNotifyFunc", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "GetResponse", slot: 5, base: "IDNS", ret: "int",
                params: &["IDNS*", "AEEDNSResponse**"],
                words: 2,
            },
            MethodInfo {
                name: "ParseDomain", slot: 6, base: "IDNS", ret: "char*",
                params: &["IDNS*", "byte*", "int*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IDNSConfig",
        base: "IQI",
        declared_in: "AEEIDNSConfig.h",
        iid: 0x0101c8af,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SelectNetwork", slot: 3, base: "IDNSConfig", ret: "int",
                params: &["IDNSConfig*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "SetServers", slot: 4, base: "IDNSConfig", ret: "int",
                params: &["IDNSConfig*", "INSockAddr*", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "GetServers", slot: 5, base: "IDNSConfig", ret: "int",
                params: &["IDNSConfig*", "INSockAddr*", "uint16*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IDNSConfig2",
        base: "IQI",
        declared_in: "AEEIDNSConfig2.h",
        iid: 0x0102fd28,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SelectNetworkEx", slot: 3, base: "IDNSConfig2", ret: "int",
                params: &["IDNSConfig2*", "int", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "SetServers", slot: 4, base: "IDNSConfig2", ret: "int",
                params: &["IDNSConfig2*", "union AEESockAddrStorage*", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "GetServers", slot: 5, base: "IDNSConfig2", ret: "int",
                params: &["IDNSConfig2*", "union AEESockAddrStorage*", "uint16*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IDatabase",
        base: "IBase",
        declared_in: "AEEDB.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "GetRecordCount", slot: 2, base: "IDatabase", ret: "uint16",
                params: &["IDatabase*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 3, base: "IDatabase", ret: "void",
                params: &["IDatabase*"],
                words: 1,
            },
            MethodInfo {
                name: "GetNextRecord", slot: 4, base: "IDatabase", ret: "IDBRecord*",
                params: &["IDatabase*"],
                words: 1,
            },
            MethodInfo {
                name: "GetRecordByID", slot: 5, base: "IDatabase", ret: "IDBRecord*",
                params: &["IDatabase*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "CreateRecord", slot: 6, base: "IDatabase", ret: "IDBRecord*",
                params: &["IDatabase*", "AEEDBField*", "int"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IDateCtl",
        base: "IBase",
        declared_in: "AEEDate.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IControl", ret: "boolean",
                params: &["IControl*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Redraw", slot: 3, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetActive", slot: 4, base: "IControl", ret: "void",
                params: &["IControl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "IsActive", slot: 5, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetRect", slot: 6, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRect", slot: 7, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "SetProperties", slot: 8, base: "IControl", ret: "void",
                params: &["IControl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 9, base: "IControl", ret: "uint32",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 10, base: "IControl", ret: "void",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetTitle", slot: 11, base: "IDateCtl", ret: "boolean",
                params: &["IDateCtl*", "char*", "uint16", "AECHAR*"],
                words: 4,
            },
            MethodInfo {
                name: "SetJulianDay", slot: 12, base: "IDateCtl", ret: "boolean",
                params: &["IDateCtl*", "int32"],
                words: 2,
            },
            MethodInfo {
                name: "GetJulianDay", slot: 13, base: "IDateCtl", ret: "int32",
                params: &["IDateCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetDate", slot: 14, base: "IDateCtl", ret: "boolean",
                params: &["IDateCtl*", "int", "int", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetDate", slot: 15, base: "IDateCtl", ret: "boolean",
                params: &["IDateCtl*", "int*", "int*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetDayOfWeek", slot: 16, base: "IDateCtl", ret: "uint16",
                params: &["IDateCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "GetDayString", slot: 17, base: "IDateCtl", ret: "AECHAR*",
                params: &["IDateCtl*", "AECHAR*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetMonthString", slot: 18, base: "IDateCtl", ret: "AECHAR*",
                params: &["IDateCtl*", "AECHAR*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetDateString", slot: 19, base: "IDateCtl", ret: "boolean",
                params: &["IDateCtl*", "AECHAR*", "int", "int*", "uint32"],
                words: 5,
            },
            MethodInfo {
                name: "EnableCommand", slot: 20, base: "IDateCtl", ret: "void",
                params: &["IDateCtl*", "boolean", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "SetActiveDayMask", slot: 21, base: "IDateCtl", ret: "void",
                params: &["IDateCtl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "SetFont", slot: 22, base: "IDateCtl", ret: "void",
                params: &["IDateCtl*", "AEEFont", "AEEFont"],
                words: 3,
            },
            MethodInfo {
                name: "GetFont", slot: 23, base: "IDateCtl", ret: "boolean",
                params: &["IDateCtl*", "AEEFont*", "AEEFont*"],
                words: 3,
            },
            MethodInfo {
                name: "SizeToFit", slot: 24, base: "IDateCtl", ret: "void",
                params: &["IDateCtl*", "AEERect*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IDialog",
        base: "IBase",
        declared_in: "AEEShell.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "SetFocus", slot: 2, base: "IDialog", ret: "int16",
                params: &["IDialog*", "int16"],
                words: 2,
            },
            MethodInfo {
                name: "GetControl", slot: 3, base: "IDialog", ret: "IControl*",
                params: &["IDialog*", "int16"],
                words: 2,
            },
            MethodInfo {
                name: "SetEventHandler", slot: 4, base: "IDialog", ret: "void",
                params: &["IDialog*", "PFNAEEEVENT", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "GetID", slot: 5, base: "IDialog", ret: "uint16",
                params: &["IDialog*"],
                words: 1,
            },
            MethodInfo {
                name: "SetProperties", slot: 6, base: "IDialog", ret: "void",
                params: &["IDialog*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 7, base: "IDialog", ret: "uint32",
                params: &["IDialog*"],
                words: 1,
            },
            MethodInfo {
                name: "Redraw", slot: 8, base: "IDialog", ret: "void",
                params: &["IDialog*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IDispatcher",
        base: "IQI",
        declared_in: "AEEIDispatcher.h",
        iid: 0x01024f18,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Resume", slot: 3, base: "IDispatcher", ret: "void",
                params: &["IDispatcher*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "Call", slot: 4, base: "IDispatcher", ret: "void",
                params: &["IDispatcher*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "Timer", slot: 5, base: "IDispatcher", ret: "void",
                params: &["IDispatcher*", "AEECallback*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetTime", slot: 6, base: "IDispatcher", ret: "uint32",
                params: &["IDispatcher*"],
                words: 1,
            },
            MethodInfo {
                name: "AtTime", slot: 7, base: "IDispatcher", ret: "void",
                params: &["IDispatcher*", "AEECallback*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Dispatch", slot: 8, base: "IDispatcher", ret: "void",
                params: &["IDispatcher*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IDisplay",
        base: "IBase",
        declared_in: "AEEIDisplay.h",
        iid: 0x01036d36,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "GetFontMetrics", slot: 2, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "AEEFont", "int*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "MeasureTextEx", slot: 3, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "AEEFont", "AECHAR*", "int", "int", "int*"],
                words: 6,
            },
            MethodInfo {
                name: "DrawText", slot: 4, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "AEEFont", "AECHAR*", "int", "int", "int", "AEERect*", "uint32"],
                words: 8,
            },
            MethodInfo {
                name: "DrawRect", slot: 5, base: "IDisplay", ret: "void",
                params: &["IDisplay*", "AEERect*", "RGBVAL", "RGBVAL", "uint32"],
                words: 5,
            },
            MethodInfo {
                name: "BitBlt", slot: 6, base: "IDisplay", ret: "void",
                params: &["IDisplay*", "int", "int", "int", "int", "void*", "int", "int", "AEERasterOp"],
                words: 9,
            },
            MethodInfo {
                name: "Update", slot: 7, base: "IDisplay", ret: "void",
                params: &["IDisplay*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "SetAnnunciators", slot: 8, base: "IDisplay", ret: "void",
                params: &["IDisplay*", "uint16", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "Backlight", slot: 9, base: "IDisplay", ret: "void",
                params: &["IDisplay*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "SetColor", slot: 10, base: "IDisplay", ret: "RGBVAL",
                params: &["IDisplay*", "AEEClrItem", "RGBVAL"],
                words: 3,
            },
            MethodInfo {
                name: "GetSymbol", slot: 11, base: "IDisplay", ret: "AECHAR",
                params: &["IDisplay*", "AEESymbol", "AEEFont"],
                words: 3,
            },
            MethodInfo {
                name: "DrawFrame", slot: 12, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "AEERect*", "AEEFrameType", "RGBVAL"],
                words: 4,
            },
            MethodInfo {
                name: "CreateDIBitmap", slot: 13, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "IDIB**", "uint8", "uint16", "uint16"],
                words: 5,
            },
            MethodInfo {
                name: "SetDestination", slot: 14, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "IBitmap*"],
                words: 2,
            },
            MethodInfo {
                name: "GetDestination", slot: 15, base: "IDisplay", ret: "IBitmap*",
                params: &["IDisplay*"],
                words: 1,
            },
            MethodInfo {
                name: "GetDeviceBitmap", slot: 16, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "IBitmap**"],
                words: 2,
            },
            MethodInfo {
                name: "SetFont", slot: 17, base: "IDisplay", ret: "IFont*",
                params: &["IDisplay*", "AEEFont", "IFont*"],
                words: 3,
            },
            MethodInfo {
                name: "SetClipRect", slot: 18, base: "IDisplay", ret: "void",
                params: &["IDisplay*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetClipRect", slot: 19, base: "IDisplay", ret: "void",
                params: &["IDisplay*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "Clone", slot: 20, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "IDisplay**"],
                words: 2,
            },
            MethodInfo {
                name: "MakeDefault", slot: 21, base: "IDisplay", ret: "void",
                params: &["IDisplay*"],
                words: 1,
            },
            MethodInfo {
                name: "IsEnabled", slot: 22, base: "IDisplay", ret: "boolean",
                params: &["IDisplay*"],
                words: 1,
            },
            MethodInfo {
                name: "NotifyEnable", slot: 23, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "CreateDIBitmapEx", slot: 24, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "IDIB**", "int", "int", "int", "int", "int"],
                words: 7,
            },
            MethodInfo {
                name: "SetPrefs", slot: 25, base: "IDisplay", ret: "int",
                params: &["IDisplay*", "char*", "int"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IEGL",
        base: "IQueryInterface",
        declared_in: "AEEGL.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "eglGetError", slot: 3, base: "IEGL", ret: "EGLint",
                params: &[],
                words: 0,
            },
            MethodInfo {
                name: "eglGetDisplay", slot: 4, base: "IEGL", ret: "EGLDisplay",
                params: &["NativeDisplayType"],
                words: 1,
            },
            MethodInfo {
                name: "eglInitialize", slot: 5, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLint*", "EGLint*"],
                words: 3,
            },
            MethodInfo {
                name: "eglTerminate", slot: 6, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay"],
                words: 1,
            },
            MethodInfo {
                name: "eglQueryString", slot: 7, base: "IEGL", ret: "char*",
                params: &["EGLDisplay", "EGLint"],
                words: 2,
            },
            MethodInfo {
                name: "eglGetProcAddress", slot: 8, base: "IEGL", ret: "void (*",
                params: &["fnptr"],
                words: 1,
            },
            MethodInfo {
                name: "eglGetConfigs", slot: 9, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLConfig*", "EGLint", "EGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "eglChooseConfig", slot: 10, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLint*", "EGLConfig*", "EGLint", "EGLint*"],
                words: 5,
            },
            MethodInfo {
                name: "eglGetConfigAttrib", slot: 11, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLConfig", "EGLint", "EGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "eglCreateWindowSurface", slot: 12, base: "IEGL", ret: "EGLSurface",
                params: &["EGLDisplay", "EGLConfig", "NativeWindowType", "EGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "eglCreatePixmapSurface", slot: 13, base: "IEGL", ret: "EGLSurface",
                params: &["EGLDisplay", "EGLConfig", "NativePixmapType", "EGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "eglCreatePbufferSurface", slot: 14, base: "IEGL", ret: "EGLSurface",
                params: &["EGLDisplay", "EGLConfig", "EGLint*"],
                words: 3,
            },
            MethodInfo {
                name: "eglDestroySurface", slot: 15, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLSurface"],
                words: 2,
            },
            MethodInfo {
                name: "eglQuerySurface", slot: 16, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLSurface", "EGLint", "EGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "eglCreateContext", slot: 17, base: "IEGL", ret: "EGLContext",
                params: &["EGLDisplay", "EGLConfig", "EGLContext", "EGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "eglDestroyContext", slot: 18, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLContext"],
                words: 2,
            },
            MethodInfo {
                name: "eglMakeCurrent", slot: 19, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLSurface", "EGLSurface", "EGLContext"],
                words: 4,
            },
            MethodInfo {
                name: "eglGetCurrentContext", slot: 20, base: "IEGL", ret: "EGLContext",
                params: &[],
                words: 0,
            },
            MethodInfo {
                name: "eglGetCurrentSurface", slot: 21, base: "IEGL", ret: "EGLSurface",
                params: &["EGLint"],
                words: 1,
            },
            MethodInfo {
                name: "eglGetCurrentDisplay", slot: 22, base: "IEGL", ret: "EGLDisplay",
                params: &[],
                words: 0,
            },
            MethodInfo {
                name: "eglQueryContext", slot: 23, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLContext", "EGLint", "EGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "eglWaitGL", slot: 24, base: "IEGL", ret: "EGLBoolean",
                params: &[],
                words: 0,
            },
            MethodInfo {
                name: "eglWaitNative", slot: 25, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLint"],
                words: 1,
            },
            MethodInfo {
                name: "eglSwapBuffers", slot: 26, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLSurface"],
                words: 2,
            },
            MethodInfo {
                name: "eglCopyBuffers", slot: 27, base: "IEGL", ret: "EGLBoolean",
                params: &["EGLDisplay", "EGLSurface", "NativePixmapType"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IEGL10",
        base: "IQueryInterface",
        declared_in: "AEEEGL10.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetError", slot: 3, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLint*"],
                words: 2,
            },
            MethodInfo {
                name: "GetDisplay", slot: 4, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "IQueryInterface*", "AEEEGLDisplay*"],
                words: 3,
            },
            MethodInfo {
                name: "Initialize", slot: 5, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLint*", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "Terminate", slot: 6, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLBoolean*"],
                words: 3,
            },
            MethodInfo {
                name: "QueryString", slot: 7, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLint", "char**"],
                words: 4,
            },
            MethodInfo {
                name: "GetConfigs", slot: 8, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig*", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "ChooseConfig", slot: 9, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLint*", "AEEEGLConfig*", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "GetConfigAttrib", slot: 10, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "CreateWindowSurface", slot: 11, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "IQueryInterface*", "AEEEGLint*", "AEEEGLSurface*"],
                words: 6,
            },
            MethodInfo {
                name: "CreatePixmapSurface", slot: 12, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "IQueryInterface*", "AEEEGLint*", "AEEEGLSurface*"],
                words: 6,
            },
            MethodInfo {
                name: "CreatePbufferSurface", slot: 13, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "AEEEGLint*", "AEEEGLSurface*"],
                words: 5,
            },
            MethodInfo {
                name: "DestroySurface", slot: 14, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*"],
                words: 4,
            },
            MethodInfo {
                name: "QuerySurface", slot: 15, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "CreateContext", slot: 16, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "AEEEGLContext", "AEEEGLint*", "AEEEGLContext*"],
                words: 6,
            },
            MethodInfo {
                name: "DestroyContext", slot: 17, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLContext", "AEEEGLBoolean*"],
                words: 4,
            },
            MethodInfo {
                name: "MakeCurrent", slot: 18, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurface", "AEEEGLContext", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "GetCurrentContext", slot: 19, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLContext*"],
                words: 2,
            },
            MethodInfo {
                name: "GetCurrentSurface", slot: 20, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLint", "AEEEGLSurface*"],
                words: 3,
            },
            MethodInfo {
                name: "GetCurrentDisplay", slot: 21, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay*"],
                words: 2,
            },
            MethodInfo {
                name: "QueryContext", slot: 22, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLContext", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "WaitGL", slot: 23, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLBoolean*"],
                words: 2,
            },
            MethodInfo {
                name: "WaitNative", slot: 24, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLint", "AEEEGLBoolean*"],
                words: 3,
            },
            MethodInfo {
                name: "SwapBuffers", slot: 25, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*"],
                words: 4,
            },
            MethodInfo {
                name: "CopyBuffers", slot: 26, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "IQueryInterface*", "AEEEGLBoolean*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IEGL11",
        base: "IEGL10",
        declared_in: "AEEEGL11.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetError", slot: 3, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLint*"],
                words: 2,
            },
            MethodInfo {
                name: "GetDisplay", slot: 4, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "IQueryInterface*", "AEEEGLDisplay*"],
                words: 3,
            },
            MethodInfo {
                name: "Initialize", slot: 5, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLint*", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "Terminate", slot: 6, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLBoolean*"],
                words: 3,
            },
            MethodInfo {
                name: "QueryString", slot: 7, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLint", "char**"],
                words: 4,
            },
            MethodInfo {
                name: "GetConfigs", slot: 8, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig*", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "ChooseConfig", slot: 9, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLint*", "AEEEGLConfig*", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "GetConfigAttrib", slot: 10, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "CreateWindowSurface", slot: 11, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "IQueryInterface*", "AEEEGLint*", "AEEEGLSurface*"],
                words: 6,
            },
            MethodInfo {
                name: "CreatePixmapSurface", slot: 12, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "IQueryInterface*", "AEEEGLint*", "AEEEGLSurface*"],
                words: 6,
            },
            MethodInfo {
                name: "CreatePbufferSurface", slot: 13, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "AEEEGLint*", "AEEEGLSurface*"],
                words: 5,
            },
            MethodInfo {
                name: "DestroySurface", slot: 14, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*"],
                words: 4,
            },
            MethodInfo {
                name: "QuerySurface", slot: 15, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "CreateContext", slot: 16, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLConfig", "AEEEGLContext", "AEEEGLint*", "AEEEGLContext*"],
                words: 6,
            },
            MethodInfo {
                name: "DestroyContext", slot: 17, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLContext", "AEEEGLBoolean*"],
                words: 4,
            },
            MethodInfo {
                name: "MakeCurrent", slot: 18, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurface", "AEEEGLContext", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "GetCurrentContext", slot: 19, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLContext*"],
                words: 2,
            },
            MethodInfo {
                name: "GetCurrentSurface", slot: 20, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLint", "AEEEGLSurface*"],
                words: 3,
            },
            MethodInfo {
                name: "GetCurrentDisplay", slot: 21, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay*"],
                words: 2,
            },
            MethodInfo {
                name: "QueryContext", slot: 22, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLContext", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "WaitGL", slot: 23, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLBoolean*"],
                words: 2,
            },
            MethodInfo {
                name: "WaitNative", slot: 24, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLint", "AEEEGLBoolean*"],
                words: 3,
            },
            MethodInfo {
                name: "SwapBuffers", slot: 25, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*"],
                words: 4,
            },
            MethodInfo {
                name: "CopyBuffers", slot: 26, base: "IEGL10", ret: "int",
                params: &["IEGL10*", "AEEEGLDisplay", "AEEEGLSurface", "IQueryInterface*", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SurfaceAttrib", slot: 27, base: "IEGL11", ret: "int",
                params: &["IEGL11*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint", "AEEEGLint", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "BindTexImage", slot: 28, base: "IEGL11", ret: "int",
                params: &["IEGL11*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "ReleaseTexImage", slot: 29, base: "IEGL11", ret: "int",
                params: &["IEGL11*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SwapInterval", slot: 30, base: "IEGL11", ret: "int",
                params: &["IEGL11*", "AEEEGLDisplay", "AEEEGLint", "AEEEGLBoolean*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IEGLGetColorBuffer",
        base: "IQueryInterface",
        declared_in: "AEEEGLGetColorBuffer.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetColorBuffer", slot: 3, base: "IEGLGetColorBuffer", ret: "int",
                params: &["IEGLGetColorBuffer*", "void**"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IEGLGetColorBufferInfo",
        base: "IQueryInterface",
        declared_in: "AEEEGLGetColorBufferInfo.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetColorBufferInfo", slot: 3, base: "IEGLGetColorBufferInfo", ret: "int",
                params: &["IEGLGetColorBufferInfo*", "AEEEGLint", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IEGLGetPowerLevel",
        base: "IQueryInterface",
        declared_in: "AEEEGLGetPowerLevel.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetPowerLevel", slot: 3, base: "IEGLGetPowerLevel", ret: "int",
                params: &["IEGLGetPowerLevel*", "int*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IEGLOESSwapInterval",
        base: "IQueryInterface",
        declared_in: "AEEEGLOESSwapInterval.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SwapInterval", slot: 3, base: "IEGLOESSwapInterval", ret: "int",
                params: &["IEGLOESSwapInterval*", "AEEEGLDisplay", "int", "AEEEGLBoolean*"],
                words: 4,
            },
            MethodInfo {
                name: "GetSwapInterval", slot: 4, base: "IEGLOESSwapInterval", ret: "int",
                params: &["IEGLOESSwapInterval*", "AEEEGLDisplay", "int*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IEGLSurfaceManip",
        base: "IEGLSurfaceManip_V1",
        declared_in: "AEEEGLSurfaceManip.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SurfaceScaleEnable", slot: 3, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SetSurfaceScale", slot: 4, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurfaceScaleRect*", "AEEEGLSurfaceScaleRect*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "GetSurfaceScale", slot: 5, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLSurfaceScaleRect*", "AEEEGLSurfaceScaleRect*", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "GetSurfaceScaleCaps", slot: 6, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurfaceScaleCaps*", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SurfaceRotateEnable", slot: 7, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SetSurfaceRotate", slot: 8, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "GetSurfaceRotate", slot: 9, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*"],
                words: 8,
            },
            MethodInfo {
                name: "GetSurfaceRotateCaps", slot: 10, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "SurfaceTransparencyEnable", slot: 11, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SetSurfaceTransparency", slot: 12, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "GetSurfaceTransparency", slot: 13, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "SetSurfaceTransparencyMap", slot: 14, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurface", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "GetSurfaceTransparencyMap", slot: 15, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLSurface*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "GetSurfaceTransparencyCaps", slot: 16, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "SurfaceColorKeyEnable", slot: 17, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SetSurfaceColorKey", slot: 18, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint", "AEEEGLint", "AEEEGLint", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "GetSurfaceColorKey", slot: 19, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLint*", "AEEEGLint*", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 8,
            },
            MethodInfo {
                name: "CreateCompositeSurface", slot: 20, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint*", "AEEEGLSurface*"],
                words: 5,
            },
            MethodInfo {
                name: "SurfaceOverlayEnable", slot: 21, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SurfaceOverlayLayerEnable", slot: 22, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "SurfaceOverlayBind", slot: 23, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurface", "AEEEGLint", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "GetSurfaceOverlayBinding", slot: 24, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurface*", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "GetSurfaceOverlay", slot: 25, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "GetSurfaceOverlayCaps", slot: 26, base: "IEGLSurfaceManip", ret: "int",
                params: &["IEGLSurfaceManip*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLCompositeSurfaceCaps*", "AEEEGLBoolean*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IEGLSurfaceManip_V1",
        base: "IQueryInterface",
        declared_in: "AEEEGLSurfaceManip.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SurfaceScaleEnable", slot: 3, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SetSurfaceScale", slot: 4, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurfaceScaleRect*", "AEEEGLSurfaceScaleRect*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "GetSurfaceScale", slot: 5, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLSurfaceScaleRect*", "AEEEGLSurfaceScaleRect*", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "GetSurfaceScaleCaps", slot: 6, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurfaceScaleCaps*", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SurfaceRotateEnable", slot: 7, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SetSurfaceRotate", slot: 8, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "GetSurfaceRotate", slot: 9, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*"],
                words: 8,
            },
            MethodInfo {
                name: "GetSurfaceRotateCaps", slot: 10, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*"],
                words: 7,
            },
            MethodInfo {
                name: "SurfaceTransparencyEnable", slot: 11, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "SetSurfaceTransparency", slot: 12, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLint", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "GetSurfaceTransparency", slot: 13, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLint*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "SetSurfaceTransparencyMap", slot: 14, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLSurface", "AEEEGLBoolean*"],
                words: 5,
            },
            MethodInfo {
                name: "GetSurfaceTransparencyMap", slot: 15, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLSurface*", "AEEEGLBoolean*"],
                words: 6,
            },
            MethodInfo {
                name: "GetSurfaceTransparencyCaps", slot: 16, base: "IEGLSurfaceManip_V1", ret: "int",
                params: &["IEGLSurfaceManip_V1*", "AEEEGLDisplay", "AEEEGLSurface", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*", "AEEEGLBoolean*"],
                words: 7,
            },
        ],
    },
    IfaceInfo {
        name: "IEnv",
        base: "IRealloc",
        declared_in: "AEEIEnv.h",
        iid: 0x0102346d,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "ErrRealloc", slot: 3, base: "IRealloc", ret: "int",
                params: &["IRealloc*", "int", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "ErrReallocName", slot: 4, base: "IRealloc", ret: "int",
                params: &["IRealloc*", "int", "void**", "char*"],
                words: 4,
            },
            MethodInfo {
                name: "ErrReallocNoZI", slot: 5, base: "IRealloc", ret: "int",
                params: &["IRealloc*", "int", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "ErrReallocNameNoZI", slot: 6, base: "IRealloc", ret: "int",
                params: &["IRealloc*", "int", "void**", "char*"],
                words: 4,
            },
            MethodInfo {
                name: "CreateInstance", slot: 7, base: "IEnv", ret: "int",
                params: &["IEnv*", "AEECLSID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AtExit", slot: 8, base: "IEnv", ret: "int",
                params: &["IEnv*", "fnptr", "void*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IFIFO",
        base: "IPort",
        declared_in: "AEEFIFO.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLastError", slot: 5, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Write", slot: 6, base: "IPort", ret: "int32",
                params: &["IPort*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Writeable", slot: 7, base: "IPort", ret: "void",
                params: &["IPort*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "IOCtl", slot: 8, base: "IPort", ret: "int",
                params: &["IPort*", "int", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Close", slot: 9, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Open", slot: 10, base: "IPort", ret: "int",
                params: &["IPort*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "OpenEx", slot: 11, base: "IFIFO", ret: "int",
                params: &["IFIFO*", "char*", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "Flush", slot: 12, base: "IFIFO", ret: "int32",
                params: &["IFIFO*", "int32"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IFile",
        base: "IAStream",
        declared_in: "AEEFile.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Readable", slot: 2, base: "IAStream", ret: "void",
                params: &["IAStream*", "fnptr", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "IAStream", ret: "int32",
                params: &["IAStream*", "void*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Cancel", slot: 4, base: "IAStream", ret: "void",
                params: &["IAStream*", "fnptr", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Write", slot: 5, base: "IFile", ret: "uint32",
                params: &["IFile*", "PACKED void*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "GetInfo", slot: 6, base: "IFile", ret: "int",
                params: &["IFile*", "FileInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "Seek", slot: 7, base: "IFile", ret: "int32",
                params: &["IFile*", "FileSeekType", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Truncate", slot: 8, base: "IFile", ret: "int",
                params: &["IFile*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetInfoEx", slot: 9, base: "IFile", ret: "int",
                params: &["IFile*", "AEEFileInfoEx*"],
                words: 2,
            },
            MethodInfo {
                name: "SetCacheSize", slot: 10, base: "IFile", ret: "int32",
                params: &["IFile*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "Map", slot: 11, base: "IFile", ret: "void*",
                params: &["IFile*", "void*", "uint32", "int", "int", "uint32"],
                words: 6,
            },
        ],
    },
    IfaceInfo {
        name: "IFileMgr",
        base: "IBase",
        declared_in: "AEEFile.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "OpenFile", slot: 2, base: "IFileMgr", ret: "IFile*",
                params: &["IFileMgr*", "char*", "OpenFileMode"],
                words: 3,
            },
            MethodInfo {
                name: "GetInfo", slot: 3, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*", "FileInfo*"],
                words: 3,
            },
            MethodInfo {
                name: "Remove", slot: 4, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "MkDir", slot: 5, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "RmDir", slot: 6, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "Test", slot: 7, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "GetFreeSpace", slot: 8, base: "IFileMgr", ret: "uint32",
                params: &["IFileMgr*", "uint32*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLastError", slot: 9, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumInit", slot: 10, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*", "boolean"],
                words: 3,
            },
            MethodInfo {
                name: "EnumNext", slot: 11, base: "IFileMgr", ret: "boolean",
                params: &["IFileMgr*", "FileInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "Rename", slot: 12, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*", "char*"],
                words: 3,
            },
            MethodInfo {
                name: "EnumNextEx", slot: 13, base: "IFileMgr", ret: "boolean",
                params: &["IFileMgr*", "AEEFileInfoEx*"],
                words: 2,
            },
            MethodInfo {
                name: "SetDescription", slot: 14, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*", "AECHAR*"],
                words: 3,
            },
            MethodInfo {
                name: "GetInfoEx", slot: 15, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*", "AEEFileInfoEx*"],
                words: 3,
            },
            MethodInfo {
                name: "Use", slot: 16, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*", "boolean"],
                words: 3,
            },
            MethodInfo {
                name: "GetFileUseInfo", slot: 17, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "AEEFileUseInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "ResolvePath", slot: 18, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*", "char*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "CheckPathAccess", slot: 19, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*", "uint32", "uint32*"],
                words: 4,
            },
            MethodInfo {
                name: "GetFreeSpaceEx", slot: 20, base: "IFileMgr", ret: "int",
                params: &["IFileMgr*", "char*", "uint32*", "uint32*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IFilePort",
        base: "IPort",
        declared_in: "AEEIFilePort.h",
        iid: 0x0101a114,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLastError", slot: 5, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Write", slot: 6, base: "IPort", ret: "int32",
                params: &["IPort*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Writeable", slot: 7, base: "IPort", ret: "void",
                params: &["IPort*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "IOCtl", slot: 8, base: "IPort", ret: "int",
                params: &["IPort*", "int", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Close", slot: 9, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Open", slot: 10, base: "IPort", ret: "int",
                params: &["IPort*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "OpenEx", slot: 11, base: "IFilePort", ret: "int",
                params: &["IFilePort*", "char*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Seek", slot: 12, base: "IFilePort", ret: "int32",
                params: &["IFilePort*", "int32", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Stat", slot: 13, base: "IFilePort", ret: "int",
                params: &["IFilePort*", "FSStat*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "StatVFS", slot: 14, base: "IFilePort", ret: "int",
                params: &["IFilePort*", "FSStatVFS*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Truncate", slot: 15, base: "IFilePort", ret: "int",
                params: &["IFilePort*", "int32"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IFileSystem",
        base: "IQI",
        declared_in: "AEEIFileSystem.h",
        iid: 0x0101adf9,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Access", slot: 3, base: "IFileSystem", ret: "int",
                params: &["IFileSystem*", "char*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "MkDir", slot: 4, base: "IFileSystem", ret: "int",
                params: &["IFileSystem*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "Rename", slot: 5, base: "IFileSystem", ret: "int",
                params: &["IFileSystem*", "char*", "char*"],
                words: 3,
            },
            MethodInfo {
                name: "RmDir", slot: 6, base: "IFileSystem", ret: "int",
                params: &["IFileSystem*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "Stat", slot: 7, base: "IFileSystem", ret: "int",
                params: &["IFileSystem*", "char*", "FSStat*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "StatVFS", slot: 8, base: "IFileSystem", ret: "int",
                params: &["IFileSystem*", "char*", "FSStatVFS*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "Truncate", slot: 9, base: "IFileSystem", ret: "int",
                params: &["IFileSystem*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Unlink", slot: 10, base: "IFileSystem", ret: "int",
                params: &["IFileSystem*", "char*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IFlip",
        base: "IQueryInterface",
        declared_in: "AEEFlip.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetFlipInfoEx", slot: 3, base: "IFlip", ret: "int",
                params: &["IFlip*", "AEEFlipInfoEx*"],
                words: 2,
            },
            MethodInfo {
                name: "GetFlipIDList", slot: 4, base: "IFlip", ret: "int",
                params: &["IFlip*", "AEEFlipInfo*", "int16*"],
                words: 3,
            },
            MethodInfo {
                name: "GetFlipPosition", slot: 5, base: "IFlip", ret: "int",
                params: &["IFlip*", "AEEFlipInfo*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "SetParm", slot: 6, base: "IFlip", ret: "int",
                params: &["IFlip*", "AEEFlipInfo*", "int16", "int32", "int32"],
                words: 5,
            },
            MethodInfo {
                name: "GetParm", slot: 7, base: "IFlip", ret: "int",
                params: &["IFlip*", "AEEFlipInfo*", "int16", "int32*", "int32*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IFont",
        base: "IQI",
        declared_in: "AEEIFont.h",
        iid: 0x01001022,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "DrawText", slot: 3, base: "IFont", ret: "int",
                params: &["IFont*", "IBitmap*", "int", "int", "AECHAR*", "int", "NativeColor", "NativeColor", "AEERect*", "uint32"],
                words: 10,
            },
            MethodInfo {
                name: "MeasureText", slot: 4, base: "IFont", ret: "int",
                params: &["IFont*", "AECHAR*", "int", "int", "int*", "int*"],
                words: 6,
            },
            MethodInfo {
                name: "GetInfo", slot: 5, base: "IFont", ret: "int",
                params: &["IFont*", "AEEFontInfo*", "int"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IForceFeed",
        base: "IQI",
        declared_in: "AEEIForceFeed.h",
        iid: 0x0101eb0b,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Write", slot: 3, base: "IForceFeed", ret: "int",
                params: &["IForceFeed*", "void*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Reset", slot: 4, base: "IForceFeed", ret: "void",
                params: &["IForceFeed*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IGLES10",
        base: "IQueryInterface",
        declared_in: "AEEGLES10.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AlphaFunc", slot: 3, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLclampf"],
                words: 3,
            },
            MethodInfo {
                name: "ClearColor", slot: 4, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampf", "AEEGLclampf", "AEEGLclampf", "AEEGLclampf"],
                words: 5,
            },
            MethodInfo {
                name: "ClearDepthf", slot: 5, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampf"],
                words: 2,
            },
            MethodInfo {
                name: "Color4f", slot: 6, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 5,
            },
            MethodInfo {
                name: "DepthRangef", slot: 7, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampf", "AEEGLclampf"],
                words: 3,
            },
            MethodInfo {
                name: "Fogf", slot: 8, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "Fogfv", slot: 9, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "Frustumf", slot: 10, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 7,
            },
            MethodInfo {
                name: "LightModelf", slot: 11, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "LightModelfv", slot: 12, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "Lightf", slot: 13, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "Lightfv", slot: 14, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "LineWidth", slot: 15, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat"],
                words: 2,
            },
            MethodInfo {
                name: "LoadMatrixf", slot: 16, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat*"],
                words: 2,
            },
            MethodInfo {
                name: "Materialf", slot: 17, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "Materialfv", slot: 18, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "MultMatrixf", slot: 19, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat*"],
                words: 2,
            },
            MethodInfo {
                name: "MultiTexCoord4f", slot: 20, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 6,
            },
            MethodInfo {
                name: "Normal3f", slot: 21, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "Orthof", slot: 22, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 7,
            },
            MethodInfo {
                name: "PointSize", slot: 23, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat"],
                words: 2,
            },
            MethodInfo {
                name: "PolygonOffset", slot: 24, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "Rotatef", slot: 25, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 5,
            },
            MethodInfo {
                name: "Scalef", slot: 26, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnvf", slot: 27, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnvfv", slot: 28, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameterf", slot: 29, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "Translatef", slot: 30, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "ActiveTexture", slot: 31, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "AlphaFuncx", slot: 32, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLclampx"],
                words: 3,
            },
            MethodInfo {
                name: "BindTexture", slot: 33, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLuint"],
                words: 3,
            },
            MethodInfo {
                name: "BlendFunc", slot: 34, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum"],
                words: 3,
            },
            MethodInfo {
                name: "Clear", slot: 35, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLbitfield"],
                words: 2,
            },
            MethodInfo {
                name: "ClearColorx", slot: 36, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampx", "AEEGLclampx", "AEEGLclampx", "AEEGLclampx"],
                words: 5,
            },
            MethodInfo {
                name: "ClearDepthx", slot: 37, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampx"],
                words: 2,
            },
            MethodInfo {
                name: "ClearStencil", slot: 38, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint"],
                words: 2,
            },
            MethodInfo {
                name: "ClientActiveTexture", slot: 39, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "Color4x", slot: 40, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 5,
            },
            MethodInfo {
                name: "ColorMask", slot: 41, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLboolean", "AEEGLboolean", "AEEGLboolean", "AEEGLboolean"],
                words: 5,
            },
            MethodInfo {
                name: "ColorPointer", slot: 42, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "CompressedTexImage2D", slot: 43, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLsizei", "AEEGLint", "AEEGLsizei", "AEEGLvoid*"],
                words: 9,
            },
            MethodInfo {
                name: "CompressedTexSubImage2D", slot: 44, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 10,
            },
            MethodInfo {
                name: "CopyTexImage2D", slot: 45, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLint"],
                words: 9,
            },
            MethodInfo {
                name: "CopyTexSubImage2D", slot: 46, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei"],
                words: 9,
            },
            MethodInfo {
                name: "CullFace", slot: 47, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "DeleteTextures", slot: 48, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "DepthFunc", slot: 49, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "DepthMask", slot: 50, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLboolean"],
                words: 2,
            },
            MethodInfo {
                name: "DepthRangex", slot: 51, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampx", "AEEGLclampx"],
                words: 3,
            },
            MethodInfo {
                name: "Disable", slot: 52, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "DisableClientState", slot: 53, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "DrawArrays", slot: 54, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLsizei"],
                words: 4,
            },
            MethodInfo {
                name: "DrawElements", slot: 55, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLsizei", "AEEGLenum", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "Enable", slot: 56, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "EnableClientState", slot: 57, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "Finish", slot: 58, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "Flush", slot: 59, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "Fogx", slot: 60, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "Fogxv", slot: 61, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "FrontFace", slot: 62, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "Frustumx", slot: 63, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 7,
            },
            MethodInfo {
                name: "GenTextures", slot: 64, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "GetError", slot: 65, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum*"],
                words: 2,
            },
            MethodInfo {
                name: "GetIntegerv", slot: 66, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint*"],
                words: 3,
            },
            MethodInfo {
                name: "GetString", slot: 67, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLubyte**"],
                words: 3,
            },
            MethodInfo {
                name: "Hint", slot: 68, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum"],
                words: 3,
            },
            MethodInfo {
                name: "LightModelx", slot: 69, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "LightModelxv", slot: 70, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "Lightx", slot: 71, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "Lightxv", slot: 72, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "LineWidthx", slot: 73, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed"],
                words: 2,
            },
            MethodInfo {
                name: "LoadIdentity", slot: 74, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "LoadMatrixx", slot: 75, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed*"],
                words: 2,
            },
            MethodInfo {
                name: "LogicOp", slot: 76, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "Materialx", slot: 77, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "Materialxv", slot: 78, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "MatrixMode", slot: 79, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "MultMatrixx", slot: 80, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed*"],
                words: 2,
            },
            MethodInfo {
                name: "MultiTexCoord4x", slot: 81, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 6,
            },
            MethodInfo {
                name: "Normal3x", slot: 82, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "NormalPointer", slot: 83, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 4,
            },
            MethodInfo {
                name: "Orthox", slot: 84, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 7,
            },
            MethodInfo {
                name: "PixelStorei", slot: 85, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint"],
                words: 3,
            },
            MethodInfo {
                name: "PointSizex", slot: 86, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed"],
                words: 2,
            },
            MethodInfo {
                name: "PolygonOffsetx", slot: 87, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "PopMatrix", slot: 88, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "PushMatrix", slot: 89, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "ReadPixels", slot: 90, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLenum", "AEEGLenum", "AEEGLvoid*"],
                words: 8,
            },
            MethodInfo {
                name: "Rotatex", slot: 91, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 5,
            },
            MethodInfo {
                name: "SampleCoverage", slot: 92, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampf", "AEEGLboolean"],
                words: 3,
            },
            MethodInfo {
                name: "SampleCoveragex", slot: 93, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampx", "AEEGLboolean"],
                words: 3,
            },
            MethodInfo {
                name: "Scalex", slot: 94, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "Scissor", slot: 95, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei"],
                words: 5,
            },
            MethodInfo {
                name: "ShadeModel", slot: 96, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "StencilFunc", slot: 97, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLuint"],
                words: 4,
            },
            MethodInfo {
                name: "StencilMask", slot: 98, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLuint"],
                words: 2,
            },
            MethodInfo {
                name: "StencilOp", slot: 99, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLenum"],
                words: 4,
            },
            MethodInfo {
                name: "TexCoordPointer", slot: 100, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "TexEnvx", slot: 101, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnvxv", slot: 102, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "TexImage2D", slot: 103, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLint", "AEEGLenum", "AEEGLenum", "AEEGLvoid*"],
                words: 10,
            },
            MethodInfo {
                name: "TexParameterx", slot: 104, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "TexSubImage2D", slot: 105, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLenum", "AEEGLenum", "AEEGLvoid*"],
                words: 10,
            },
            MethodInfo {
                name: "Translatex", slot: 106, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "VertexPointer", slot: 107, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "Viewport", slot: 108, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IGLES10Ext",
        base: "IQueryInterface",
        declared_in: "AEEGLES10Ext.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "QueryMatrixxOES", slot: 3, base: "IGLES10Ext", ret: "int",
                params: &["IGLES10Ext*", "AEEGLfixed*", "AEEGLint*", "AEEGLbitfield*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IGLES11",
        base: "IGLES10",
        declared_in: "AEEGLES11.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AlphaFunc", slot: 3, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLclampf"],
                words: 3,
            },
            MethodInfo {
                name: "ClearColor", slot: 4, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampf", "AEEGLclampf", "AEEGLclampf", "AEEGLclampf"],
                words: 5,
            },
            MethodInfo {
                name: "ClearDepthf", slot: 5, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampf"],
                words: 2,
            },
            MethodInfo {
                name: "Color4f", slot: 6, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 5,
            },
            MethodInfo {
                name: "DepthRangef", slot: 7, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampf", "AEEGLclampf"],
                words: 3,
            },
            MethodInfo {
                name: "Fogf", slot: 8, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "Fogfv", slot: 9, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "Frustumf", slot: 10, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 7,
            },
            MethodInfo {
                name: "LightModelf", slot: 11, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "LightModelfv", slot: 12, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "Lightf", slot: 13, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "Lightfv", slot: 14, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "LineWidth", slot: 15, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat"],
                words: 2,
            },
            MethodInfo {
                name: "LoadMatrixf", slot: 16, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat*"],
                words: 2,
            },
            MethodInfo {
                name: "Materialf", slot: 17, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "Materialfv", slot: 18, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "MultMatrixf", slot: 19, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat*"],
                words: 2,
            },
            MethodInfo {
                name: "MultiTexCoord4f", slot: 20, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 6,
            },
            MethodInfo {
                name: "Normal3f", slot: 21, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "Orthof", slot: 22, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 7,
            },
            MethodInfo {
                name: "PointSize", slot: 23, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat"],
                words: 2,
            },
            MethodInfo {
                name: "PolygonOffset", slot: 24, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "Rotatef", slot: 25, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 5,
            },
            MethodInfo {
                name: "Scalef", slot: 26, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnvf", slot: 27, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnvfv", slot: 28, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameterf", slot: 29, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "Translatef", slot: 30, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "ActiveTexture", slot: 31, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "AlphaFuncx", slot: 32, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLclampx"],
                words: 3,
            },
            MethodInfo {
                name: "BindTexture", slot: 33, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLuint"],
                words: 3,
            },
            MethodInfo {
                name: "BlendFunc", slot: 34, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum"],
                words: 3,
            },
            MethodInfo {
                name: "Clear", slot: 35, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLbitfield"],
                words: 2,
            },
            MethodInfo {
                name: "ClearColorx", slot: 36, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampx", "AEEGLclampx", "AEEGLclampx", "AEEGLclampx"],
                words: 5,
            },
            MethodInfo {
                name: "ClearDepthx", slot: 37, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampx"],
                words: 2,
            },
            MethodInfo {
                name: "ClearStencil", slot: 38, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint"],
                words: 2,
            },
            MethodInfo {
                name: "ClientActiveTexture", slot: 39, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "Color4x", slot: 40, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 5,
            },
            MethodInfo {
                name: "ColorMask", slot: 41, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLboolean", "AEEGLboolean", "AEEGLboolean", "AEEGLboolean"],
                words: 5,
            },
            MethodInfo {
                name: "ColorPointer", slot: 42, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "CompressedTexImage2D", slot: 43, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLsizei", "AEEGLint", "AEEGLsizei", "AEEGLvoid*"],
                words: 9,
            },
            MethodInfo {
                name: "CompressedTexSubImage2D", slot: 44, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 10,
            },
            MethodInfo {
                name: "CopyTexImage2D", slot: 45, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLint"],
                words: 9,
            },
            MethodInfo {
                name: "CopyTexSubImage2D", slot: 46, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei"],
                words: 9,
            },
            MethodInfo {
                name: "CullFace", slot: 47, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "DeleteTextures", slot: 48, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "DepthFunc", slot: 49, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "DepthMask", slot: 50, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLboolean"],
                words: 2,
            },
            MethodInfo {
                name: "DepthRangex", slot: 51, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampx", "AEEGLclampx"],
                words: 3,
            },
            MethodInfo {
                name: "Disable", slot: 52, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "DisableClientState", slot: 53, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "DrawArrays", slot: 54, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLsizei"],
                words: 4,
            },
            MethodInfo {
                name: "DrawElements", slot: 55, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLsizei", "AEEGLenum", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "Enable", slot: 56, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "EnableClientState", slot: 57, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "Finish", slot: 58, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "Flush", slot: 59, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "Fogx", slot: 60, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "Fogxv", slot: 61, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "FrontFace", slot: 62, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "Frustumx", slot: 63, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 7,
            },
            MethodInfo {
                name: "GenTextures", slot: 64, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "GetError", slot: 65, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum*"],
                words: 2,
            },
            MethodInfo {
                name: "GetIntegerv", slot: 66, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint*"],
                words: 3,
            },
            MethodInfo {
                name: "GetString", slot: 67, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLubyte**"],
                words: 3,
            },
            MethodInfo {
                name: "Hint", slot: 68, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum"],
                words: 3,
            },
            MethodInfo {
                name: "LightModelx", slot: 69, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "LightModelxv", slot: 70, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "Lightx", slot: 71, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "Lightxv", slot: 72, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "LineWidthx", slot: 73, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed"],
                words: 2,
            },
            MethodInfo {
                name: "LoadIdentity", slot: 74, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "LoadMatrixx", slot: 75, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed*"],
                words: 2,
            },
            MethodInfo {
                name: "LogicOp", slot: 76, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "Materialx", slot: 77, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "Materialxv", slot: 78, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "MatrixMode", slot: 79, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "MultMatrixx", slot: 80, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed*"],
                words: 2,
            },
            MethodInfo {
                name: "MultiTexCoord4x", slot: 81, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 6,
            },
            MethodInfo {
                name: "Normal3x", slot: 82, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "NormalPointer", slot: 83, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 4,
            },
            MethodInfo {
                name: "Orthox", slot: 84, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 7,
            },
            MethodInfo {
                name: "PixelStorei", slot: 85, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint"],
                words: 3,
            },
            MethodInfo {
                name: "PointSizex", slot: 86, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed"],
                words: 2,
            },
            MethodInfo {
                name: "PolygonOffsetx", slot: 87, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "PopMatrix", slot: 88, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "PushMatrix", slot: 89, base: "IGLES10", ret: "int",
                params: &["IGLES10*"],
                words: 1,
            },
            MethodInfo {
                name: "ReadPixels", slot: 90, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLenum", "AEEGLenum", "AEEGLvoid*"],
                words: 8,
            },
            MethodInfo {
                name: "Rotatex", slot: 91, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 5,
            },
            MethodInfo {
                name: "SampleCoverage", slot: 92, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampf", "AEEGLboolean"],
                words: 3,
            },
            MethodInfo {
                name: "SampleCoveragex", slot: 93, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLclampx", "AEEGLboolean"],
                words: 3,
            },
            MethodInfo {
                name: "Scalex", slot: 94, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "Scissor", slot: 95, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei"],
                words: 5,
            },
            MethodInfo {
                name: "ShadeModel", slot: 96, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "StencilFunc", slot: 97, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLuint"],
                words: 4,
            },
            MethodInfo {
                name: "StencilMask", slot: 98, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLuint"],
                words: 2,
            },
            MethodInfo {
                name: "StencilOp", slot: 99, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLenum"],
                words: 4,
            },
            MethodInfo {
                name: "TexCoordPointer", slot: 100, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "TexEnvx", slot: 101, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnvxv", slot: 102, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "TexImage2D", slot: 103, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLint", "AEEGLenum", "AEEGLenum", "AEEGLvoid*"],
                words: 10,
            },
            MethodInfo {
                name: "TexParameterx", slot: 104, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "TexSubImage2D", slot: 105, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLenum", "AEEGLenum", "AEEGLvoid*"],
                words: 10,
            },
            MethodInfo {
                name: "Translatex", slot: 106, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "VertexPointer", slot: 107, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "Viewport", slot: 108, base: "IGLES10", ret: "int",
                params: &["IGLES10*", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei"],
                words: 5,
            },
            MethodInfo {
                name: "ClipPlanef", slot: 109, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "GetClipPlanef", slot: 110, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfloat eqn"],
                words: 3,
            },
            MethodInfo {
                name: "GetFloatv", slot: 111, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "GetLightfv", slot: 112, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "GetMaterialfv", slot: 113, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexEnvfv", slot: 114, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexParameterfv", slot: 115, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "PointParameterf", slot: 116, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "PointParameterfv", slot: 117, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "TexParameterfv", slot: 118, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "BindBuffer", slot: 119, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLuint"],
                words: 3,
            },
            MethodInfo {
                name: "BufferData", slot: 120, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLsizeiptr", "AEEGLvoid*", "AEEGLenum"],
                words: 5,
            },
            MethodInfo {
                name: "BufferSubData", slot: 121, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLintptr", "AEEGLsizeiptr", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "ClipPlanex", slot: 122, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "Color4ub", slot: 123, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLubyte", "AEEGLubyte", "AEEGLubyte", "AEEGLubyte"],
                words: 5,
            },
            MethodInfo {
                name: "DeleteBuffers", slot: 124, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "GetBooleanv", slot: 125, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLboolean*"],
                words: 3,
            },
            MethodInfo {
                name: "GetBufferParameteriv", slot: 126, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "GetClipPlanex", slot: 127, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfixed eqn"],
                words: 3,
            },
            MethodInfo {
                name: "GenBuffers", slot: 128, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "GetFixedv", slot: 129, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "GetLightxv", slot: 130, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "GetMaterialxv", slot: 131, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "GetPointerv", slot: 132, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetTexEnviv", slot: 133, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexEnvxv", slot: 134, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexParameteriv", slot: 135, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexParameterxv", slot: 136, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "IsBuffer", slot: 137, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLuint", "AEEGLboolean*"],
                words: 3,
            },
            MethodInfo {
                name: "IsEnabled", slot: 138, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLboolean*"],
                words: 3,
            },
            MethodInfo {
                name: "IsTexture", slot: 139, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLuint", "AEEGLboolean*"],
                words: 3,
            },
            MethodInfo {
                name: "PointParameterx", slot: 140, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "PointParameterxv", slot: 141, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "TexEnvi", slot: 142, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLint"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnviv", slot: 143, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameteri", slot: 144, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLint"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameteriv", slot: 145, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameterxv", slot: 146, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "PointSizePointerOES", slot: 147, base: "IGLES11", ret: "int",
                params: &["IGLES11*", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IGLES11Ext",
        base: "IQueryInterface",
        declared_in: "AEEGLES11Ext.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "CurrentPaletteMatrixOES", slot: 3, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLuint"],
                words: 2,
            },
            MethodInfo {
                name: "LoadPaletteFromModelViewMatrixOES", slot: 4, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*"],
                words: 1,
            },
            MethodInfo {
                name: "MatrixIndexPointerOES", slot: 5, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "WeightPointerOES", slot: 6, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "DrawTexsOES", slot: 7, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLshort", "AEEGLshort", "AEEGLshort", "AEEGLshort", "AEEGLshort"],
                words: 6,
            },
            MethodInfo {
                name: "DrawTexiOES", slot: 8, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLint"],
                words: 6,
            },
            MethodInfo {
                name: "DrawTexxOES", slot: 9, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 6,
            },
            MethodInfo {
                name: "DrawTexsvOES", slot: 10, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLshort*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawTexivOES", slot: 11, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLint*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawTexxvOES", slot: 12, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLfixed*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawTexfOES", slot: 13, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 6,
            },
            MethodInfo {
                name: "DrawTexfvOES", slot: 14, base: "IGLES11Ext", ret: "int",
                params: &["IGLES11Ext*", "AEEGLfloat*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IGLES11ExtPak",
        base: "IQueryInterface",
        declared_in: "AEEGLES11ExtPak.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetTexGenfv", slot: 3, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexGeniv", slot: 4, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexGenxv", slot: 5, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "TexGenf", slot: 6, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "TexGeni", slot: 7, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLint"],
                words: 4,
            },
            MethodInfo {
                name: "TexGenx", slot: 8, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "TexGenfv", slot: 9, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "TexGeniv", slot: 10, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "TexGenxv", slot: 11, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "BlendEquation", slot: 12, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "BlendFuncSeparate", slot: 13, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLenum", "AEEGLenum"],
                words: 5,
            },
            MethodInfo {
                name: "BlendEquationSeparate", slot: 14, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum"],
                words: 3,
            },
            MethodInfo {
                name: "BindFramebufferOES", slot: 15, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLuint"],
                words: 3,
            },
            MethodInfo {
                name: "BindRenderbufferOES", slot: 16, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLuint"],
                words: 3,
            },
            MethodInfo {
                name: "CheckFramebufferStatusOES", slot: 17, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum*"],
                words: 3,
            },
            MethodInfo {
                name: "DeleteFramebuffersOES", slot: 18, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "DeleteRenderbuffersOES", slot: 19, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "FramebufferRenderbufferOES", slot: 20, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLenum", "AEEGLuint"],
                words: 5,
            },
            MethodInfo {
                name: "FramebufferTexture2DOES", slot: 21, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLenum", "AEEGLuint", "AEEGLint"],
                words: 6,
            },
            MethodInfo {
                name: "GenerateMipmapOES", slot: 22, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "GenFramebuffersOES", slot: 23, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "GenRenderbuffersOES", slot: 24, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "GetFramebufferAttachmentParameterivOES", slot: 25, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 5,
            },
            MethodInfo {
                name: "GetRenderbufferParameterivOES", slot: 26, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "IsFramebufferOES", slot: 27, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLuint", "AEEGLboolean*"],
                words: 3,
            },
            MethodInfo {
                name: "IsRenderbufferOES", slot: 28, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLuint", "AEEGLboolean*"],
                words: 3,
            },
            MethodInfo {
                name: "RenderbufferStorageOES", slot: 29, base: "IGLES11ExtPak", ret: "int",
                params: &["IGLES11ExtPak*", "AEEGLenum", "AEEGLenum", "AEEGLsizei", "AEEGLsizei"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IGLESImageonExt",
        base: "IGLESImageonExt_V1",
        declared_in: "AEEGLESImageonEXT.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "PointSizePointerOES", slot: 3, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 4,
            },
            MethodInfo {
                name: "BlendEquationSeparateEXT", slot: 4, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum"],
                words: 3,
            },
            MethodInfo {
                name: "BlendFuncSeparateEXT", slot: 5, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLenum", "AEEGLenum"],
                words: 5,
            },
            MethodInfo {
                name: "BlendEquationEXT", slot: 6, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "BindBufferQUALCOMM", slot: 7, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLuint"],
                words: 3,
            },
            MethodInfo {
                name: "DeleteBuffersQUALCOMM", slot: 8, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "GenBuffersQUALCOMM", slot: 9, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "BufferDataQUALCOMM", slot: 10, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLsizeiptrARB", "AEEGLvoid*", "AEEGLenum"],
                words: 5,
            },
            MethodInfo {
                name: "BufferSubDataQUALCOMM", slot: 11, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLintptrARB", "AEEGLsizeiptrARB", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "IsBufferQUALCOMM", slot: 12, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLuint", "AEEGLboolean*"],
                words: 3,
            },
            MethodInfo {
                name: "BufferDataATI", slot: 13, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLsizei", "AEEGLsizei", "AEEGLsizeiptrARB", "AEEGLvoid*", "AEEGLenum"],
                words: 7,
            },
            MethodInfo {
                name: "MeshListATI", slot: 14, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLint", "AEEGLenum mode", "AEEGLsizei count", "AEEGLenum type", "AEEGLvoid*indices"],
                words: 6,
            },
            MethodInfo {
                name: "DrawVertexBufferObjectATI", slot: 15, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*"],
                words: 1,
            },
            MethodInfo {
                name: "GetPointerv", slot: 16, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "TexEnvi", slot: 17, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLint"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnviv", slot: 18, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameteri", slot: 19, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLint"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameteriv", slot: 20, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameterfv", slot: 21, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameterxv", slot: 22, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
            MethodInfo {
                name: "GetMaterialfv", slot: 23, base: "IGLESImageonExt", ret: "int",
                params: &["IGLESImageonExt*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexParameteriv", slot: 24, base: "IGLESImageonExt", ret: "int",
                params: &["IGLESImageonExt*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexParameterfv", slot: 25, base: "IGLESImageonExt", ret: "int",
                params: &["IGLESImageonExt*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "GetTexParameterxv", slot: 26, base: "IGLESImageonExt", ret: "int",
                params: &["IGLESImageonExt*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IGLESImageonExt_V1",
        base: "IQueryInterface",
        declared_in: "AEEGLESImageonEXT.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "PointSizePointerOES", slot: 3, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 4,
            },
            MethodInfo {
                name: "BlendEquationSeparateEXT", slot: 4, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum"],
                words: 3,
            },
            MethodInfo {
                name: "BlendFuncSeparateEXT", slot: 5, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLenum", "AEEGLenum"],
                words: 5,
            },
            MethodInfo {
                name: "BlendEquationEXT", slot: 6, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "BindBufferQUALCOMM", slot: 7, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLuint"],
                words: 3,
            },
            MethodInfo {
                name: "DeleteBuffersQUALCOMM", slot: 8, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "GenBuffersQUALCOMM", slot: 9, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLsizei", "AEEGLuint*"],
                words: 3,
            },
            MethodInfo {
                name: "BufferDataQUALCOMM", slot: 10, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLsizeiptrARB", "AEEGLvoid*", "AEEGLenum"],
                words: 5,
            },
            MethodInfo {
                name: "BufferSubDataQUALCOMM", slot: 11, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLintptrARB", "AEEGLsizeiptrARB", "AEEGLvoid*"],
                words: 5,
            },
            MethodInfo {
                name: "IsBufferQUALCOMM", slot: 12, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLuint", "AEEGLboolean*"],
                words: 3,
            },
            MethodInfo {
                name: "BufferDataATI", slot: 13, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLsizei", "AEEGLsizei", "AEEGLsizeiptrARB", "AEEGLvoid*", "AEEGLenum"],
                words: 7,
            },
            MethodInfo {
                name: "MeshListATI", slot: 14, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLint", "AEEGLenum mode", "AEEGLsizei count", "AEEGLenum type", "AEEGLvoid*indices"],
                words: 6,
            },
            MethodInfo {
                name: "DrawVertexBufferObjectATI", slot: 15, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*"],
                words: 1,
            },
            MethodInfo {
                name: "GetPointerv", slot: 16, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "TexEnvi", slot: 17, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLint"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnviv", slot: 18, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameteri", slot: 19, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLint"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameteriv", slot: 20, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLint*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameterfv", slot: 21, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 4,
            },
            MethodInfo {
                name: "TexParameterxv", slot: 22, base: "IGLESImageonExt_V1", ret: "int",
                params: &["IGLESImageonExt_V1*", "AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IGetline",
        base: "IPeek",
        declared_in: "AEESource.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "Peek", slot: 5, base: "IPeek", ret: "char*",
                params: &["IPeek*", "int32", "int32*", "int32*"],
                words: 4,
            },
            MethodInfo {
                name: "Peekable", slot: 6, base: "IPeek", ret: "void",
                params: &["IPeek*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "Advance", slot: 7, base: "IPeek", ret: "void",
                params: &["IPeek*", "int32"],
                words: 2,
            },
            MethodInfo {
                name: "GetLine", slot: 8, base: "IGetline", ret: "int32",
                params: &["IGetline*", "GetLine*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "UngetLine", slot: 9, base: "IGetline", ret: "void",
                params: &["IGetline*", "int32"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IGraphics",
        base: "IBase",
        declared_in: "AEEGraphics.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "SetBackground", slot: 2, base: "IGraphics", ret: "RGBVAL",
                params: &["IGraphics*", "uint8", "uint8", "uint8"],
                words: 4,
            },
            MethodInfo {
                name: "GetBackground", slot: 3, base: "IGraphics", ret: "void",
                params: &["IGraphics*", "uint8*", "uint8*", "uint8*"],
                words: 4,
            },
            MethodInfo {
                name: "SetColor", slot: 4, base: "IGraphics", ret: "RGBVAL",
                params: &["IGraphics*", "uint8", "uint8", "uint8", "uint8"],
                words: 5,
            },
            MethodInfo {
                name: "GetColor", slot: 5, base: "IGraphics", ret: "void",
                params: &["IGraphics*", "uint8*", "uint8*", "uint8*", "uint8*"],
                words: 5,
            },
            MethodInfo {
                name: "SetFillMode", slot: 6, base: "IGraphics", ret: "boolean",
                params: &["IGraphics*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "GetFillMode", slot: 7, base: "IGraphics", ret: "boolean",
                params: &["IGraphics*"],
                words: 1,
            },
            MethodInfo {
                name: "SetFillColor", slot: 8, base: "IGraphics", ret: "RGBVAL",
                params: &["IGraphics*", "uint8", "uint8", "uint8", "uint8"],
                words: 5,
            },
            MethodInfo {
                name: "GetFillColor", slot: 9, base: "IGraphics", ret: "void",
                params: &["IGraphics*", "uint8*", "uint8*", "uint8*", "uint8*"],
                words: 5,
            },
            MethodInfo {
                name: "SetPointSize", slot: 10, base: "IGraphics", ret: "uint8",
                params: &["IGraphics*", "uint8"],
                words: 2,
            },
            MethodInfo {
                name: "GetPointSize", slot: 11, base: "IGraphics", ret: "uint8",
                params: &["IGraphics*"],
                words: 1,
            },
            MethodInfo {
                name: "SetClip", slot: 12, base: "IGraphics", ret: "boolean",
                params: &["IGraphics*", "AEEClip*", "uint8"],
                words: 3,
            },
            MethodInfo {
                name: "GetClip", slot: 13, base: "IGraphics", ret: "boolean",
                params: &["IGraphics*", "AEEClip*"],
                words: 2,
            },
            MethodInfo {
                name: "SetViewport", slot: 14, base: "IGraphics", ret: "boolean",
                params: &["IGraphics*", "AEERect*", "uint8"],
                words: 3,
            },
            MethodInfo {
                name: "GetViewport", slot: 15, base: "IGraphics", ret: "boolean",
                params: &["IGraphics*", "AEERect*", "boolean*"],
                words: 3,
            },
            MethodInfo {
                name: "ClearViewport", slot: 16, base: "IGraphics", ret: "void",
                params: &["IGraphics*"],
                words: 1,
            },
            MethodInfo {
                name: "SetPaintMode", slot: 17, base: "IGraphics", ret: "AEEPaintMode",
                params: &["IGraphics*", "AEEPaintMode"],
                words: 2,
            },
            MethodInfo {
                name: "GetPaintMode", slot: 18, base: "IGraphics", ret: "AEEPaintMode",
                params: &["IGraphics*"],
                words: 1,
            },
            MethodInfo {
                name: "GetColorDepth", slot: 19, base: "IGraphics", ret: "uint8",
                params: &["IGraphics*"],
                words: 1,
            },
            MethodInfo {
                name: "DrawPoint", slot: 20, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEEPoint*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawLine", slot: 21, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEELine*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawRect", slot: 22, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawCircle", slot: 23, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEECircle*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawArc", slot: 24, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEEArc*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawPie", slot: 25, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEEPie*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawEllipse", slot: 26, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEEEllipse*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawTriangle", slot: 27, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEETriangle*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawPolygon", slot: 28, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEEPolygon*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawPolyline", slot: 29, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEEPolyline*"],
                words: 2,
            },
            MethodInfo {
                name: "ClearRect", slot: 30, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "EnableDoubleBuffer", slot: 31, base: "IGraphics", ret: "boolean",
                params: &["IGraphics*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "Update", slot: 32, base: "IGraphics", ret: "void",
                params: &["IGraphics*"],
                words: 1,
            },
            MethodInfo {
                name: "Translate", slot: 33, base: "IGraphics", ret: "void",
                params: &["IGraphics*", "int16", "int16"],
                words: 3,
            },
            MethodInfo {
                name: "Pan", slot: 34, base: "IGraphics", ret: "void",
                params: &["IGraphics*", "int16", "int16"],
                words: 3,
            },
            MethodInfo {
                name: "StretchBlt", slot: 35, base: "IGraphics", ret: "void",
                params: &["IGraphics*", "int16", "int16", "int16", "int16", "void*", "AEERasterOp"],
                words: 7,
            },
            MethodInfo {
                name: "SetAlgorithmHint", slot: 36, base: "IGraphics", ret: "void",
                params: &["IGraphics*", "AEEAlgorithmHint"],
                words: 2,
            },
            MethodInfo {
                name: "GetAlgorithmHint", slot: 37, base: "IGraphics", ret: "AEEAlgorithmHint",
                params: &["IGraphics*"],
                words: 1,
            },
            MethodInfo {
                name: "SetDestination", slot: 38, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "IBitmap*"],
                words: 2,
            },
            MethodInfo {
                name: "GetDestination", slot: 39, base: "IGraphics", ret: "IBitmap*",
                params: &["IGraphics*"],
                words: 1,
            },
            MethodInfo {
                name: "SetStrokeStyle", slot: 40, base: "IGraphics", ret: "AEEStrokeStyle",
                params: &["IGraphics*", "AEEStrokeStyle"],
                words: 2,
            },
            MethodInfo {
                name: "GetStrokeStyle", slot: 41, base: "IGraphics", ret: "AEEStrokeStyle",
                params: &["IGraphics*"],
                words: 1,
            },
            MethodInfo {
                name: "DrawEllipticalArc", slot: 42, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEERect*", "int16", "int16"],
                words: 4,
            },
            MethodInfo {
                name: "DrawRoundRectangle", slot: 43, base: "IGraphics", ret: "int",
                params: &["IGraphics*", "AEERect*", "int16", "int16"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IHID",
        base: "IQI",
        declared_in: "AEEIHID.h",
        iid: 0x0106c38d,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "CreateDevice", slot: 3, base: "IHID", ret: "AEEResult",
                params: &["IHID*", "int", "IHIDDevice**"],
                words: 3,
            },
            MethodInfo {
                name: "GetDeviceInfo", slot: 4, base: "IHID", ret: "AEEResult",
                params: &["IHID*", "int", "AEEHIDDeviceInfo*"],
                words: 3,
            },
            MethodInfo {
                name: "GetNextConnectEvent", slot: 5, base: "IHID", ret: "AEEResult",
                params: &["IHID*", "int*", "int*", "boolean*"],
                words: 4,
            },
            MethodInfo {
                name: "RegisterForConnectEvents", slot: 6, base: "IHID", ret: "AEEResult",
                params: &["IHID*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetConnectedDevices", slot: 7, base: "IHID", ret: "AEEResult",
                params: &["IHID*", "int", "int*", "int", "int*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IHIDDevice",
        base: "IQI",
        declared_in: "AEEIHIDDevice.h",
        iid: 0x0106c38e,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetDeviceInfo", slot: 3, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "AEEHIDDeviceInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "GetDeviceStatus", slot: 4, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "int*"],
                words: 2,
            },
            MethodInfo {
                name: "RegisterForStatusChange", slot: 5, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetButtonInfo", slot: 6, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "int", "AEEHIDButtonInfo*"],
                words: 3,
            },
            MethodInfo {
                name: "GetNumberOfButtons", slot: 7, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "int*"],
                words: 2,
            },
            MethodInfo {
                name: "RegisterForButtonEvent", slot: 8, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetNextButtonEvent", slot: 9, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "AEEHIDButtonInfo*", "uint32*", "boolean*"],
                words: 4,
            },
            MethodInfo {
                name: "GetPositionState", slot: 10, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "AEEHIDPositionInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "GetMinPositionInfo", slot: 11, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "AEEHIDPositionInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "GetMaxPositionInfo", slot: 12, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "AEEHIDPositionInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "GetAxesInfo", slot: 13, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "AEEHIDPositionInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "RegisterForPositionChange", slot: 14, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "SetExclusiveLevel", slot: 15, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "GetExclusiveLevel", slot: 16, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "int*"],
                words: 2,
            },
            MethodInfo {
                name: "Rumble", slot: 17, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "int", "int"],
                words: 3,
            },
            MethodInfo {
                name: "GetRumbleStatus", slot: 18, base: "IHIDDevice", ret: "AEEResult",
                params: &["IHIDDevice*", "int*", "int*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IHash",
        base: "IBase",
        declared_in: "AEESecurity.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Update", slot: 2, base: "IHash", ret: "void",
                params: &["IHash*", "byte*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "GetResult", slot: 3, base: "IHash", ret: "int",
                params: &["IHash*", "byte*", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "Restart", slot: 4, base: "IHash", ret: "void",
                params: &["IHash*"],
                words: 1,
            },
            MethodInfo {
                name: "SetKey", slot: 5, base: "IHash", ret: "int",
                params: &["IHash*", "byte*", "int"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IHashCtx",
        base: "IQI",
        declared_in: "AEEIHashCtx.h",
        iid: 0x0102cb09,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Init", slot: 3, base: "IHashCtx", ret: "void",
                params: &["IHashCtx*", "void*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Update", slot: 4, base: "IHashCtx", ret: "void",
                params: &["IHashCtx*", "void*", "int", "void*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "Final", slot: 5, base: "IHashCtx", ret: "int",
                params: &["IHashCtx*", "void*", "int", "byte*", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "SetKey", slot: 6, base: "IHashCtx", ret: "int",
                params: &["IHashCtx*", "void*", "int", "byte*", "int"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IHeap",
        base: "IBase",
        declared_in: "AEEHeap.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Malloc", slot: 2, base: "IHeap", ret: "void*",
                params: &["IHeap*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "Realloc", slot: 3, base: "IHeap", ret: "void*",
                params: &["IHeap*", "void*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Free", slot: 4, base: "IHeap", ret: "void",
                params: &["IHeap*", "void*"],
                words: 2,
            },
            MethodInfo {
                name: "StrDup", slot: 5, base: "IHeap", ret: "AECHAR*",
                params: &["IHeap*", "AECHAR*"],
                words: 2,
            },
            MethodInfo {
                name: "CheckAvail", slot: 6, base: "IHeap", ret: "boolean",
                params: &["IHeap*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetMemStats", slot: 7, base: "IHeap", ret: "uint32",
                params: &["IHeap*"],
                words: 1,
            },
            MethodInfo {
                name: "GetModuleMemStats", slot: 8, base: "IHeap", ret: "int",
                params: &["IHeap*", "AEECLSID", "uint32*", "uint32*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IHtmlViewer",
        base: "IControl",
        declared_in: "AEEHtmlViewer.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IControl", ret: "boolean",
                params: &["IControl*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Redraw", slot: 3, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetActive", slot: 4, base: "IControl", ret: "void",
                params: &["IControl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "IsActive", slot: 5, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetRect", slot: 6, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRect", slot: 7, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "SetProperties", slot: 8, base: "IControl", ret: "void",
                params: &["IControl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 9, base: "IControl", ret: "uint32",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 10, base: "IControl", ret: "void",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetOption", slot: 11, base: "IHtmlViewer", ret: "int",
                params: &["IHtmlViewer*", "int32", "HVOpt"],
                words: 3,
            },
            MethodInfo {
                name: "GetOption", slot: 12, base: "IHtmlViewer", ret: "int",
                params: &["IHtmlViewer*", "int32", "HVOpt*"],
                words: 3,
            },
            MethodInfo {
                name: "LoadStream", slot: 13, base: "IHtmlViewer", ret: "void",
                params: &["IHtmlViewer*", "IAStream*"],
                words: 2,
            },
            MethodInfo {
                name: "LoadSource", slot: 14, base: "IHtmlViewer", ret: "void",
                params: &["IHtmlViewer*", "ISource*"],
                words: 2,
            },
            MethodInfo {
                name: "SetNotifyFn", slot: 15, base: "IHtmlViewer", ret: "void",
                params: &["IHtmlViewer*", "PFNHVIEWNOTIFY", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "SetData", slot: 16, base: "IHtmlViewer", ret: "void",
                params: &["IHtmlViewer*", "char*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "IsBusy", slot: 17, base: "IHtmlViewer", ret: "boolean",
                params: &["IHtmlViewer*"],
                words: 1,
            },
            MethodInfo {
                name: "Stop", slot: 18, base: "IHtmlViewer", ret: "void",
                params: &["IHtmlViewer*"],
                words: 1,
            },
            MethodInfo {
                name: "FindElem", slot: 19, base: "IHtmlViewer", ret: "int32",
                params: &["IHtmlViewer*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetElemText", slot: 20, base: "IHtmlViewer", ret: "int32",
                params: &["IHtmlViewer*", "int32", "char*", "int32"],
                words: 4,
            },
            MethodInfo {
                name: "GetAttrValue", slot: 21, base: "IHtmlViewer", ret: "int32",
                params: &["IHtmlViewer*", "int32", "char*", "char*", "int32"],
                words: 5,
            },
            MethodInfo {
                name: "SetViewState", slot: 22, base: "IHtmlViewer", ret: "void",
                params: &["IHtmlViewer*", "PCSTR"],
                words: 2,
            },
            MethodInfo {
                name: "GetViewState", slot: 23, base: "IHtmlViewer", ret: "int",
                params: &["IHtmlViewer*", "char*", "int"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IImage",
        base: "IBase",
        declared_in: "AEEIImage.h",
        iid: 0x01013110,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Draw", slot: 2, base: "IImage", ret: "void",
                params: &["IImage*", "int", "int"],
                words: 3,
            },
            MethodInfo {
                name: "DrawFrame", slot: 3, base: "IImage", ret: "void",
                params: &["IImage*", "int", "int", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetInfo", slot: 4, base: "IImage", ret: "void",
                params: &["IImage*", "AEEImageInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "SetParm", slot: 5, base: "IImage", ret: "void",
                params: &["IImage*", "int", "int", "int"],
                words: 4,
            },
            MethodInfo {
                name: "Start", slot: 6, base: "IImage", ret: "void",
                params: &["IImage*", "int", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Stop", slot: 7, base: "IImage", ret: "void",
                params: &["IImage*"],
                words: 1,
            },
            MethodInfo {
                name: "SetStream", slot: 8, base: "IImage", ret: "void",
                params: &["IImage*", "IAStream*"],
                words: 2,
            },
            MethodInfo {
                name: "HandleEvent", slot: 9, base: "IImage", ret: "boolean",
                params: &["IImage*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Notify", slot: 10, base: "IImage", ret: "void",
                params: &["IImage*", "PFNIMAGEINFO", "void*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IImageCtl",
        base: "IBase",
        declared_in: "AEEImageCtl.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IControl", ret: "boolean",
                params: &["IControl*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Redraw", slot: 3, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetActive", slot: 4, base: "IControl", ret: "void",
                params: &["IControl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "IsActive", slot: 5, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetRect", slot: 6, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRect", slot: 7, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "SetProperties", slot: 8, base: "IControl", ret: "void",
                params: &["IControl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 9, base: "IControl", ret: "uint32",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 10, base: "IControl", ret: "void",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetImage", slot: 11, base: "IImageCtl", ret: "void",
                params: &["IImageCtl*", "IImage*"],
                words: 2,
            },
            MethodInfo {
                name: "SetRedraw", slot: 12, base: "IImageCtl", ret: "void",
                params: &["IImageCtl*", "PFNNOTIFY", "void*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IImageDecoder",
        base: "IQI",
        declared_in: "AEEIImageDecoder.h",
        iid: 0x01026e20,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetBitmap", slot: 3, base: "IImageDecoder", ret: "int",
                params: &["IImageDecoder*", "IBitmap**"],
                words: 2,
            },
            MethodInfo {
                name: "GetRop", slot: 4, base: "IImageDecoder", ret: "int",
                params: &["IImageDecoder*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IItemStore",
        base: "IQI",
        declared_in: "AEEIItemStore.h",
        iid: 0x01060f39,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Get", slot: 3, base: "IItemStore", ret: "int",
                params: &["IItemStore*", "char*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "Set", slot: 4, base: "IItemStore", ret: "int",
                params: &["IItemStore*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "OnChange", slot: 5, base: "IItemStore", ret: "int",
                params: &["IItemStore*", "ISignal*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IJoystick",
        base: "IQueryInterface",
        declared_in: "AEEJoystick.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SetParm", slot: 3, base: "IJoystick", ret: "int",
                params: &["IJoystick*", "int16", "int32", "int32"],
                words: 4,
            },
            MethodInfo {
                name: "GetParm", slot: 4, base: "IJoystick", ret: "int",
                params: &["IJoystick*", "int16", "int32*"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 5, base: "IJoystick", ret: "int",
                params: &["IJoystick*", "int16*", "int16*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IKDDIPHONESYSTEM",
        base: "IQI",
        declared_in: "KDDIPHONESYSTEM.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetCallInfo", slot: 3, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*", "KDDICallInfoType"],
                words: 2,
            },
            MethodInfo {
                name: "GetChargeInfo", slot: 4, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*", "KDDICallChargeType"],
                words: 2,
            },
            MethodInfo {
                name: "GetHandSetType", slot: 5, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "GetHistory", slot: 6, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*", "KDDIHistoryType", "KDDIHistory*", "uint16"],
                words: 4,
            },
            MethodInfo {
                name: "GetIntensity", slot: 7, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*"],
                words: 1,
            },
            MethodInfo {
                name: "GetMannerMode", slot: 8, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*"],
                words: 1,
            },
            MethodInfo {
                name: "GetMaxVolumeLevel", slot: 9, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*"],
                words: 1,
            },
            MethodInfo {
                name: "GetMyProfile", slot: 10, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*", "KDDIAddrFieldType", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "GetNumberOfHistories", slot: 11, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*", "KDDIHistoryType"],
                words: 2,
            },
            MethodInfo {
                name: "GetPowerSupplyLevel", slot: 12, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*"],
                words: 1,
            },
            MethodInfo {
                name: "GetSubscriberID", slot: 13, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "GetVibrationStatus", slot: 14, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*", "KDDIVibrationType"],
                words: 2,
            },
            MethodInfo {
                name: "GetVolumeLevel", slot: 15, base: "IKDDIPHONESYSTEM", ret: "int",
                params: &["IKDDIPHONESYSTEM*", "KDDINotificationType"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IKDDIPPP",
        base: "IQI",
        declared_in: "KDDIPPP.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SetInfo", slot: 3, base: "IKDDIPPP", ret: "int",
                params: &["IKDDIPPP*", "char*", "char*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "ILicense",
        base: "IBase",
        declared_in: "AEELicense.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "IsExpired", slot: 2, base: "ILicense", ret: "boolean",
                params: &["ILicense*"],
                words: 1,
            },
            MethodInfo {
                name: "GetInfo", slot: 3, base: "ILicense", ret: "AEELicenseType",
                params: &["ILicense*", "uint32*"],
                words: 2,
            },
            MethodInfo {
                name: "SetUsesRemaining", slot: 4, base: "ILicense", ret: "int",
                params: &["ILicense*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetPurchaseInfo", slot: 5, base: "ILicense", ret: "AEEPriceType",
                params: &["ILicense*", "AEELicenseType*", "uint32*", "uint32*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "ILocalStorage",
        base: "IQI",
        declared_in: "AEEILocalStorage.h",
        iid: 0x0102fa3e,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Lock", slot: 3, base: "ILocalStorage", ret: "void",
                params: &["ILocalStorage*"],
                words: 1,
            },
            MethodInfo {
                name: "Unlock", slot: 4, base: "ILocalStorage", ret: "void",
                params: &["ILocalStorage*"],
                words: 1,
            },
            MethodInfo {
                name: "Get", slot: 5, base: "ILocalStorage", ret: "int",
                params: &["ILocalStorage*", "uint32", "LSEntry**"],
                words: 3,
            },
            MethodInfo {
                name: "Add", slot: 6, base: "ILocalStorage", ret: "int",
                params: &["ILocalStorage*", "uint32", "LSEntry*"],
                words: 3,
            },
            MethodInfo {
                name: "Remove", slot: 7, base: "ILocalStorage", ret: "void",
                params: &["ILocalStorage*", "LSEntry*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ILogger",
        base: "IBase",
        declared_in: "AEELogger.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "SetParam", slot: 2, base: "ILogger", ret: "int",
                params: &["ILogger*", "AEELogParamType", "uint32", "void*"],
                words: 4,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "ILogger", ret: "int",
                params: &["ILogger*", "AEELogParamType", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "PutRecord", slot: 4, base: "ILogger", ret: "int",
                params: &["ILogger*", "AEELogBucketType", "AEELogRecord*"],
                words: 3,
            },
            MethodInfo {
                name: "Printf", slot: 5, base: "ILogger", ret: "int",
                params: &["ILogger*", "AEELogBucketType", "char*", "uint16", "char*", "..."],
                words: 6,
            },
            MethodInfo {
                name: "PutMsg", slot: 6, base: "ILogger", ret: "int",
                params: &["ILogger*", "AEELogBucketType", "char*", "uint16", "char*", "uint8", "uint32 args"],
                words: 7,
            },
            MethodInfo {
                name: "PutItem", slot: 7, base: "ILogger", ret: "int",
                params: &["ILogger*", "AEELogBucketType", "AEELogItemType", "uint16", "uint8*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IMcastSession",
        base: "IModel",
        declared_in: "AEEIMcastSession.h",
        iid: 0x01031dfc,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddListener", slot: 3, base: "IModel", ret: "int",
                params: &["IModel*", "ModelListener*"],
                words: 2,
            },
            MethodInfo {
                name: "Notify", slot: 4, base: "IModel", ret: "void",
                params: &["IModel*", "ModelEvent*"],
                words: 2,
            },
            MethodInfo {
                name: "Join", slot: 5, base: "IMcastSession", ret: "int",
                params: &["IMcastSession*", "void*"],
                words: 2,
            },
            MethodInfo {
                name: "Leave", slot: 6, base: "IMcastSession", ret: "int",
                params: &["IMcastSession*"],
                words: 1,
            },
            MethodInfo {
                name: "GetGroupAddr", slot: 7, base: "IMcastSession", ret: "int",
                params: &["IMcastSession*", "void*", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "SelectNetworkEx", slot: 8, base: "IMcastSession", ret: "int",
                params: &["IMcastSession*", "int", "int16"],
                words: 3,
            },
            MethodInfo {
                name: "AddRetryCB", slot: 9, base: "IMcastSession", ret: "int",
                params: &["IMcastSession*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "ExtFunc", slot: 10, base: "IMcastSession", ret: "int",
                params: &["IMcastSession*", "AEECLSID", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IMedia",
        base: "IQI",
        declared_in: "AEEIMedia.h",
        iid: 0x01005500,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "RegisterNotify", slot: 3, base: "IMedia", ret: "int",
                params: &["IMedia*", "PFNMEDIANOTIFY", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "SetMediaParm", slot: 4, base: "IMedia", ret: "int",
                params: &["IMedia*", "int", "int32", "int32"],
                words: 4,
            },
            MethodInfo {
                name: "GetMediaParm", slot: 5, base: "IMedia", ret: "int",
                params: &["IMedia*", "int", "int32*", "int32*"],
                words: 4,
            },
            MethodInfo {
                name: "Play", slot: 6, base: "IMedia", ret: "int",
                params: &["IMedia*"],
                words: 1,
            },
            MethodInfo {
                name: "Record", slot: 7, base: "IMedia", ret: "int",
                params: &["IMedia*"],
                words: 1,
            },
            MethodInfo {
                name: "Stop", slot: 8, base: "IMedia", ret: "int",
                params: &["IMedia*"],
                words: 1,
            },
            MethodInfo {
                name: "Seek", slot: 9, base: "IMedia", ret: "int",
                params: &["IMedia*", "AEEMediaSeek", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Pause", slot: 10, base: "IMedia", ret: "int",
                params: &["IMedia*"],
                words: 1,
            },
            MethodInfo {
                name: "Resume", slot: 11, base: "IMedia", ret: "int",
                params: &["IMedia*"],
                words: 1,
            },
            MethodInfo {
                name: "GetTotalTime", slot: 12, base: "IMedia", ret: "int",
                params: &["IMedia*"],
                words: 1,
            },
            MethodInfo {
                name: "GetState", slot: 13, base: "IMedia", ret: "int",
                params: &["IMedia*", "boolean*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IMediaUtil",
        base: "IQueryInterface",
        declared_in: "AEEMediaUtil.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "CreateMedia", slot: 3, base: "IMediaUtil", ret: "int",
                params: &["IMediaUtil*", "AEEMediaData*", "IMedia**"],
                words: 3,
            },
            MethodInfo {
                name: "EncodeMedia", slot: 4, base: "IMediaUtil", ret: "int",
                params: &["IMediaUtil*", "AEEMediaEncodeResult*", "AEECLSID", "AEEMediaEncodeInfo*", "AEECallback*"],
                words: 5,
            },
            MethodInfo {
                name: "CreateMediaEx", slot: 5, base: "IMediaUtil", ret: "int",
                params: &["IMediaUtil*", "AEEMediaCreateInfo*", "IMedia**"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IMemAStream",
        base: "IAStream",
        declared_in: "AEE.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Readable", slot: 2, base: "IAStream", ret: "void",
                params: &["IAStream*", "fnptr", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "IAStream", ret: "int32",
                params: &["IAStream*", "void*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Cancel", slot: 4, base: "IAStream", ret: "void",
                params: &["IAStream*", "fnptr", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Set", slot: 5, base: "IMemAStream", ret: "void",
                params: &["IMemAStream*", "byte*", "uint32", "uint32", "boolean"],
                words: 5,
            },
            MethodInfo {
                name: "SetEx", slot: 6, base: "IMemAStream", ret: "void",
                params: &["IMemAStream*", "byte*", "uint32", "uint32", "PFNNOTIFY", "void*"],
                words: 6,
            },
        ],
    },
    IfaceInfo {
        name: "IMenuCtl",
        base: "IBase",
        declared_in: "AEEMenu.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IControl", ret: "boolean",
                params: &["IControl*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Redraw", slot: 3, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetActive", slot: 4, base: "IControl", ret: "void",
                params: &["IControl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "IsActive", slot: 5, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetRect", slot: 6, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRect", slot: 7, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "SetProperties", slot: 8, base: "IControl", ret: "void",
                params: &["IControl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 9, base: "IControl", ret: "uint32",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 10, base: "IControl", ret: "void",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetTitle", slot: 11, base: "IMenuCtl", ret: "boolean",
                params: &["IMenuCtl*", "char*", "uint16", "AECHAR*"],
                words: 4,
            },
            MethodInfo {
                name: "AddItem", slot: 12, base: "IMenuCtl", ret: "boolean",
                params: &["IMenuCtl*", "char*", "uint16", "uint16", "AECHAR*", "uint32"],
                words: 6,
            },
            MethodInfo {
                name: "AddItemEx", slot: 13, base: "IMenuCtl", ret: "boolean",
                params: &["IMenuCtl*", "CtlAddItem*"],
                words: 2,
            },
            MethodInfo {
                name: "GetItemData", slot: 14, base: "IMenuCtl", ret: "boolean",
                params: &["IMenuCtl*", "uint16", "uint32*"],
                words: 3,
            },
            MethodInfo {
                name: "DeleteItem", slot: 15, base: "IMenuCtl", ret: "boolean",
                params: &["IMenuCtl*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "DeleteAll", slot: 16, base: "IMenuCtl", ret: "boolean",
                params: &["IMenuCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetSel", slot: 17, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "GetSel", slot: 18, base: "IMenuCtl", ret: "uint16",
                params: &["IMenuCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "EnableCommand", slot: 19, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "SetItemText", slot: 20, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "uint16", "char*", "uint16", "AECHAR*"],
                words: 5,
            },
            MethodInfo {
                name: "SetItemTime", slot: 21, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "uint16", "uint16", "uint16"],
                words: 4,
            },
            MethodInfo {
                name: "GetItemTime", slot: 22, base: "IMenuCtl", ret: "int",
                params: &["IMenuCtl*", "uint16", "uint16*"],
                words: 3,
            },
            MethodInfo {
                name: "SetStyle", slot: 23, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "AEEItemStyle*", "AEEItemStyle*"],
                words: 3,
            },
            MethodInfo {
                name: "SetColors", slot: 24, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "AEEMenuColors*"],
                words: 2,
            },
            MethodInfo {
                name: "MoveItem", slot: 25, base: "IMenuCtl", ret: "int16",
                params: &["IMenuCtl*", "uint16", "int"],
                words: 3,
            },
            MethodInfo {
                name: "GetItemCount", slot: 26, base: "IMenuCtl", ret: "int",
                params: &["IMenuCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "GetItemID", slot: 27, base: "IMenuCtl", ret: "uint16",
                params: &["IMenuCtl*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "GetItem", slot: 28, base: "IMenuCtl", ret: "boolean",
                params: &["IMenuCtl*", "uint16", "CtlAddItem*"],
                words: 3,
            },
            MethodInfo {
                name: "SetItem", slot: 29, base: "IMenuCtl", ret: "boolean",
                params: &["IMenuCtl*", "uint16", "uint16", "CtlAddItem*"],
                words: 4,
            },
            MethodInfo {
                name: "Sort", slot: 30, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "AEEMenuSortType", "PFNMENUSORTCOMPARE", "void*"],
                words: 4,
            },
            MethodInfo {
                name: "SetSelEx", slot: 31, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "uint16", "boolean"],
                words: 3,
            },
            MethodInfo {
                name: "EnumSelInit", slot: 32, base: "IMenuCtl", ret: "boolean",
                params: &["IMenuCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumNextSel", slot: 33, base: "IMenuCtl", ret: "uint16",
                params: &["IMenuCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "GetStyle", slot: 34, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "AEEItemStyle*", "AEEItemStyle*"],
                words: 3,
            },
            MethodInfo {
                name: "GetColors", slot: 35, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "AEEMenuColors*"],
                words: 2,
            },
            MethodInfo {
                name: "GetItemRect", slot: 36, base: "IMenuCtl", ret: "void",
                params: &["IMenuCtl*", "uint16", "AEERect*"],
                words: 3,
            },
            MethodInfo {
                name: "SetOwnerDrawCB", slot: 37, base: "IMenuCtl", ret: "int",
                params: &["IMenuCtl*", "PFNOWNERDRAWCB", "void*", "uint16"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IMod",
        base: "IQI",
        declared_in: "AEEIMod.h",
        iid: 0x0102bba1,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Init", slot: 3, base: "IMod", ret: "int",
                params: &["IMod*"],
                words: 1,
            },
            MethodInfo {
                name: "CreateInstance", slot: 4, base: "IMod", ret: "int",
                params: &["IMod*", "AEECLSID", "IPrivSet*", "void**"],
                words: 4,
            },
            MethodInfo {
                name: "Exit", slot: 5, base: "IMod", ret: "void",
                params: &["IMod*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IModel",
        base: "IQI",
        declared_in: "AEEIModel.h",
        iid: 0x0101593a,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddListener", slot: 3, base: "IModel", ret: "int",
                params: &["IModel*", "ModelListener*"],
                words: 2,
            },
            MethodInfo {
                name: "Notify", slot: 4, base: "IModel", ret: "void",
                params: &["IModel*", "ModelEvent*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IModule",
        base: "IBase",
        declared_in: "AEE.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "CreateInstance", slot: 2, base: "IModule", ret: "int",
                params: &["IModule*", "IShell*", "AEECLSID", "void**"],
                words: 4,
            },
            MethodInfo {
                name: "FreeResources", slot: 3, base: "IModule", ret: "void",
                params: &["IModule*", "IHeap*", "IFileMgr*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IMultipartyCall",
        base: "IQueryInterface",
        declared_in: "AEETelephone.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddParty", slot: 3, base: "IMultipartyCall", ret: "int",
                params: &["IMultipartyCall*", "ICall*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveParty", slot: 4, base: "IMultipartyCall", ret: "int",
                params: &["IMultipartyCall*", "ICall*"],
                words: 2,
            },
            MethodInfo {
                name: "DisconnectParty", slot: 5, base: "IMultipartyCall", ret: "int",
                params: &["IMultipartyCall*", "ICall*"],
                words: 2,
            },
            MethodInfo {
                name: "GetNextCall", slot: 6, base: "IMultipartyCall", ret: "int",
                params: &["IMultipartyCall*", "ICall*", "ICall**"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "INetMgr",
        base: "INotifier",
        declared_in: "AEENet.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "SetMask", slot: 2, base: "INotifier", ret: "void",
                params: &["INotifier*", "uint32*"],
                words: 2,
            },
            MethodInfo {
                name: "GetHostByName", slot: 3, base: "INetMgr", ret: "void",
                params: &["INetMgr*", "AEEDNSResult*", "char*", "AEECallback*"],
                words: 4,
            },
            MethodInfo {
                name: "GetLastError", slot: 4, base: "INetMgr", ret: "int",
                params: &["INetMgr*"],
                words: 1,
            },
            MethodInfo {
                name: "OpenSocket", slot: 5, base: "INetMgr", ret: "ISocket*",
                params: &["INetMgr*", "NetSocket"],
                words: 2,
            },
            MethodInfo {
                name: "NetStatus", slot: 6, base: "INetMgr", ret: "NetState",
                params: &["INetMgr*", "AEENetStats*"],
                words: 2,
            },
            MethodInfo {
                name: "GetMyIPAddr", slot: 7, base: "INetMgr", ret: "INAddr",
                params: &["INetMgr*"],
                words: 1,
            },
            MethodInfo {
                name: "SetLinger", slot: 8, base: "INetMgr", ret: "uint16",
                params: &["INetMgr*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "OnEvent", slot: 9, base: "INetMgr", ret: "int",
                params: &["INetMgr*", "PFNNETMGREVENT", "void*", "boolean"],
                words: 4,
            },
            MethodInfo {
                name: "SetOpt", slot: 10, base: "INetMgr", ret: "int",
                params: &["INetMgr*", "int", "void*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetOpt", slot: 11, base: "INetMgr", ret: "int",
                params: &["INetMgr*", "int", "void*", "int*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "INetUtils",
        base: "IQI",
        declared_in: "AEEINetUtils.h",
        iid: 0x010329cf,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetDefaultNetwork", slot: 3, base: "INetUtils", ret: "int",
                params: &["INetUtils*", "int16", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "GetPDPCount", slot: 4, base: "INetUtils", ret: "int",
                params: &["INetUtils*", "uint16*"],
                words: 2,
            },
            MethodInfo {
                name: "GetPDPInfo", slot: 5, base: "INetUtils", ret: "int",
                params: &["INetUtils*", "uint16", "AEEPDPInfo*"],
                words: 3,
            },
            MethodInfo {
                name: "ExtFunc", slot: 6, base: "INetUtils", ret: "int",
                params: &["INetUtils*", "AEECLSID", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "INetwork",
        base: "IQI",
        declared_in: "AEEINetwork.h",
        iid: 0x01035f45,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SelectNetworkEx", slot: 3, base: "INetwork", ret: "int",
                params: &["INetwork*", "int", "int16"],
                words: 3,
            },
            MethodInfo {
                name: "OnEvent", slot: 4, base: "INetwork", ret: "int",
                params: &["INetwork*", "int", "PFNNETWORKEVENT", "void*", "boolean"],
                words: 5,
            },
            MethodInfo {
                name: "NetStatus", slot: 5, base: "INetwork", ret: "int",
                params: &["INetwork*", "AEENetStatus*", "AEENetStats*"],
                words: 3,
            },
            MethodInfo {
                name: "Start", slot: 6, base: "INetwork", ret: "int",
                params: &["INetwork*"],
                words: 1,
            },
            MethodInfo {
                name: "AddRetryCB", slot: 7, base: "INetwork", ret: "int",
                params: &["INetwork*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetMyIPAddrs", slot: 8, base: "INetwork", ret: "int",
                params: &["INetwork*", "IPAddr aipaAddr", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "SetLinger", slot: 9, base: "INetwork", ret: "int",
                params: &["INetwork*", "uint16", "uint16*"],
                words: 3,
            },
            MethodInfo {
                name: "SetDormancyTimeout", slot: 10, base: "INetwork", ret: "int",
                params: &["INetwork*", "uint8"],
                words: 2,
            },
            MethodInfo {
                name: "SetOpt", slot: 11, base: "INetwork", ret: "int",
                params: &["INetwork*", "int", "void*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetOpt", slot: 12, base: "INetwork", ret: "int",
                params: &["INetwork*", "int", "void*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetLastNetDownReason", slot: 13, base: "INetwork", ret: "int",
                params: &["INetwork*", "AEENetDownReason*"],
                words: 2,
            },
            MethodInfo {
                name: "IsQoSAware", slot: 14, base: "INetwork", ret: "int",
                params: &["INetwork*", "boolean*"],
                words: 2,
            },
            MethodInfo {
                name: "ExtFunc", slot: 15, base: "INetwork", ret: "int",
                params: &["INetwork*", "AEECLSID", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "INotifier",
        base: "IBase",
        declared_in: "AEE.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "SetMask", slot: 2, base: "INotifier", ret: "void",
                params: &["INotifier*", "uint32*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IOBEXCli",
        base: "IQI",
        declared_in: "AEEIOBEXCli.h",
        iid: 0x01053c89,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetSupportedTransports", slot: 3, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "AEEOBEXTransport*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "SetTransport", slot: 4, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "AEEOBEXTransport", "char*"],
                words: 3,
            },
            MethodInfo {
                name: "CreateHeaderList", slot: 5, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "IOBEXHeader**"],
                words: 2,
            },
            MethodInfo {
                name: "Authenticate", slot: 6, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "AEEOBEXCharSet", "uint8*", "int", "boolean", "boolean"],
                words: 6,
            },
            MethodInfo {
                name: "SendCmd", slot: 7, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "AEEOBEXRequestType", "IOBEXHeader*", "AEEOBEXFlags", "boolean"],
                words: 5,
            },
            MethodInfo {
                name: "OnEventNotify", slot: 8, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetEvent", slot: 9, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "AEEOBEXEventType*", "IOBEXHeader**", "AEEOBEXCmdStatus*", "AEEOBEXFlags*"],
                words: 5,
            },
            MethodInfo {
                name: "SetUidPassword", slot: 10, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "uint8*", "int", "uint8*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "GetAuthChallenge", slot: 11, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "boolean*", "boolean*", "AEEOBEXCharSet*", "uint8*", "int", "int*"],
                words: 7,
            },
            MethodInfo {
                name: "GetAuthResponse", slot: 12, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "AEEOBEXAuthResp*"],
                words: 2,
            },
            MethodInfo {
                name: "GetParams", slot: 13, base: "IOBEXCli", ret: "int",
                params: &["IOBEXCli*", "AEEOBEXParamType", "uint32*", "uint8*", "int", "int*"],
                words: 6,
            },
        ],
    },
    IfaceInfo {
        name: "IOBEXHeader",
        base: "IQI",
        declared_in: "AEEIOBEXHeader.h",
        iid: 0x01058640,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddString", slot: 3, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "AEEOBEXHeaderID", "AECHAR*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "AddU32", slot: 4, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "AEEOBEXHeaderID", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "AddByteSeq", slot: 5, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "AEEOBEXHeaderID", "uint8*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetU32", slot: 6, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "AEEOBEXHeaderID", "uint32*"],
                words: 3,
            },
            MethodInfo {
                name: "GetByteSeq", slot: 7, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "AEEOBEXHeaderID", "int", "uint8*", "int", "int*"],
                words: 6,
            },
            MethodInfo {
                name: "GetString", slot: 8, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "AEEOBEXHeaderID", "AECHAR*", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "GetList", slot: 9, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "AEEOBEXHeader*", "int", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "GetU32AtIndex", slot: 10, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "int", "uint32*"],
                words: 3,
            },
            MethodInfo {
                name: "GetByteSeqAtIndex", slot: 11, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "int", "int", "uint8*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "GetStringAtIndex", slot: 12, base: "IOBEXHeader", ret: "int",
                params: &["IOBEXHeader*", "int", "AECHAR*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IOBEXSrv",
        base: "IQI",
        declared_in: "AEEIOBEXSrv.h",
        iid: 0x01053c88,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetSupportedTransports", slot: 3, base: "IOBEXSrv", ret: "int",
                params: &["IOBEXSrv*", "AEEOBEXTransport*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "Register", slot: 4, base: "IOBEXSrv", ret: "int",
                params: &["IOBEXSrv*", "AEEOBEXTransport", "uint8*", "int", "char*"],
                words: 5,
            },
            MethodInfo {
                name: "Deregister", slot: 5, base: "IOBEXSrv", ret: "int",
                params: &["IOBEXSrv*"],
                words: 1,
            },
            MethodInfo {
                name: "OnEventNotify", slot: 6, base: "IOBEXSrv", ret: "int",
                params: &["IOBEXSrv*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetEvent", slot: 7, base: "IOBEXSrv", ret: "int",
                params: &["IOBEXSrv*", "AEEOBEXEventType*", "AEEOBEXCmdStatus*", "IOBEXSrvSession**"],
                words: 4,
            },
            MethodInfo {
                name: "GetParams", slot: 8, base: "IOBEXSrv", ret: "int",
                params: &["IOBEXSrv*", "AEEOBEXParamType", "uint32*", "uint8*", "int", "int*"],
                words: 6,
            },
        ],
    },
    IfaceInfo {
        name: "IOBEXSrvSession",
        base: "IQI",
        declared_in: "AEEIOBEXSrvSession.h",
        iid: 0x01058559,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "CreateHeaderList", slot: 3, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*", "IOBEXHeader**"],
                words: 2,
            },
            MethodInfo {
                name: "Authenticate", slot: 4, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*", "AEEOBEXCharSet", "uint8*", "int", "boolean", "boolean"],
                words: 6,
            },
            MethodInfo {
                name: "OnEventNotify", slot: 5, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetEvent", slot: 6, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*", "AEEOBEXEventType*", "IOBEXHeader**", "AEEOBEXEvent*"],
                words: 4,
            },
            MethodInfo {
                name: "SendResponse", slot: 7, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*", "AEEOBEXRequestType", "IOBEXHeader*", "AEEOBEXCmdStatus", "AEEOBEXFlags"],
                words: 5,
            },
            MethodInfo {
                name: "Disconnect", slot: 8, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*"],
                words: 1,
            },
            MethodInfo {
                name: "SetUidPassword", slot: 9, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*", "uint8*", "int", "uint8*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "GetAuthChallenge", slot: 10, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*", "boolean*", "boolean*", "AEEOBEXCharSet*", "uint8*", "int", "int*"],
                words: 7,
            },
            MethodInfo {
                name: "GetAuthResponse", slot: 11, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*", "AEEOBEXAuthResp*"],
                words: 2,
            },
            MethodInfo {
                name: "GetParams", slot: 12, base: "IOBEXSrvSession", ret: "int",
                params: &["IOBEXSrvSession*", "AEEOBEXParamType", "uint32*", "uint8*", "int", "int*"],
                words: 6,
            },
        ],
    },
    IfaceInfo {
        name: "IParameters",
        base: "IParametersRO",
        declared_in: "AEEIParameters.h",
        iid: 0x0102d13b,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParametersRO", ret: "int",
                params: &["IParametersRO*", "int", "void*", "unsigned*"],
                words: 4,
            },
            MethodInfo {
                name: "SetParam", slot: 4, base: "IParameters", ret: "int",
                params: &["IParameters*", "int", "void*", "unsigned"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IParameters1",
        base: "IQI",
        declared_in: "AEEIParameters1.h",
        iid: 0x0104047f,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "SetParam", slot: 4, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IParametersRO",
        base: "IQI",
        declared_in: "AEEIParameters.h",
        iid: 0x0102d13c,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParametersRO", ret: "int",
                params: &["IParametersRO*", "int", "void*", "unsigned*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IPeek",
        base: "ISource",
        declared_in: "AEEIPeek.h",
        iid: 0x0100101c,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "Peek", slot: 5, base: "IPeek", ret: "char*",
                params: &["IPeek*", "int32", "int32*", "int32*"],
                words: 4,
            },
            MethodInfo {
                name: "Peekable", slot: 6, base: "IPeek", ret: "void",
                params: &["IPeek*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "Advance", slot: 7, base: "IPeek", ret: "void",
                params: &["IPeek*", "int32"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IPhoneCtl",
        base: "IQueryInterface",
        declared_in: "AEETelephone.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "PhoneCtl_Command", slot: 3, base: "IPhoneCtl", ret: "int",
                params: &["IPhoneCtl*", "uint32", "AEECallback*", "void*", "uint32", "void*", "uint32"],
                words: 7,
            },
            MethodInfo {
                name: "SuppsCallOp", slot: 4, base: "IPhoneCtl", ret: "int",
                params: &["IPhoneCtl*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "IsSuppsCallOp", slot: 5, base: "IPhoneCtl", ret: "int",
                params: &["IPhoneCtl*", "char*", "uint32*"],
                words: 3,
            },
            MethodInfo {
                name: "GetSuppsTrans", slot: 6, base: "IPhoneCtl", ret: "int",
                params: &["IPhoneCtl*", "AEETSuppsTransId", "ISuppsTrans**"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IPort",
        base: "ISource",
        declared_in: "AEEIPort.h",
        iid: 0x01001018,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLastError", slot: 5, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Write", slot: 6, base: "IPort", ret: "int32",
                params: &["IPort*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Writeable", slot: 7, base: "IPort", ret: "void",
                params: &["IPort*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "IOCtl", slot: 8, base: "IPort", ret: "int",
                params: &["IPort*", "int", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Close", slot: 9, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Open", slot: 10, base: "IPort", ret: "int",
                params: &["IPort*", "char*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IPosDet",
        base: "IQIBASE",
        declared_in: "AEEPosDet.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQIBASE", ret: "int",
                params: &["IQIBASE*", "AEECLSID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetSectorInfo", slot: 3, base: "IPosDet", ret: "int",
                params: &["IPosDet*", "AEESectorInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "SetGPSConfig", slot: 4, base: "IPosDet", ret: "int",
                params: &["IPosDet*", "AEEGPSConfig*"],
                words: 2,
            },
            MethodInfo {
                name: "GetGPSConfig", slot: 5, base: "IPosDet", ret: "int",
                params: &["IPosDet*", "AEEGPSConfig*"],
                words: 2,
            },
            MethodInfo {
                name: "GetGPSInfo", slot: 6, base: "IPosDet", ret: "int",
                params: &["IPosDet*", "AEEGPSReq", "AEEGPSAccuracy", "AEEGPSInfo*", "AEECallback*"],
                words: 5,
            },
            MethodInfo {
                name: "GetOrientation", slot: 7, base: "IPosDet", ret: "int",
                params: &["IPosDet*", "AEEOrientationReq", "void*", "AEECallback*"],
                words: 4,
            },
            MethodInfo {
                name: "GetSectorInfoEx", slot: 8, base: "IPosDet", ret: "int",
                params: &["IPosDet*", "AEESectorInfo*", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "ExtractPositionInfo", slot: 9, base: "IPosDet", ret: "int",
                params: &["IPosDet*", "AEEGPSInfo*", "AEEPositionInfoEx*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IPrivSet",
        base: "IQI",
        declared_in: "AEEIPrivSet.h",
        iid: 0x0102f959,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "CheckPrivileges", slot: 3, base: "IPrivSet", ret: "int",
                params: &["IPrivSet*", "AEEPRIVID*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "GetPrivileges", slot: 4, base: "IPrivSet", ret: "int",
                params: &["IPrivSet*", "AEEPRIVID*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "CreateSubset", slot: 5, base: "IPrivSet", ret: "int",
                params: &["IPrivSet*", "AEEPRIVID*", "int", "IPrivSet**"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IPubKey",
        base: "IParameters1",
        declared_in: "AEEIPubKey.h",
        iid: 0x01034502,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "SetParam", slot: 4, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "Encrypt", slot: 5, base: "IPubKey", ret: "void",
                params: &["IPubKey*", "uint8*", "int", "uint8*", "int*", "int", "int*", "AEECallback*"],
                words: 8,
            },
            MethodInfo {
                name: "Decrypt", slot: 6, base: "IPubKey", ret: "void",
                params: &["IPubKey*", "uint8*", "int", "uint8*", "int*", "int", "int*", "AEECallback*"],
                words: 8,
            },
            MethodInfo {
                name: "Sign", slot: 7, base: "IPubKey", ret: "void",
                params: &["IPubKey*", "uint8*", "int", "uint8*", "int*", "int", "int*", "AEECallback*"],
                words: 8,
            },
            MethodInfo {
                name: "Verify", slot: 8, base: "IPubKey", ret: "void",
                params: &["IPubKey*", "uint8*", "int", "uint8*", "int", "int", "int*", "AEECallback*"],
                words: 8,
            },
        ],
    },
    IfaceInfo {
        name: "IPubKeyEng",
        base: "IParameters1",
        declared_in: "AEEIPubKeyEng.h",
        iid: 0x01034503,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "SetParam", slot: 4, base: "IParameters1", ret: "int",
                params: &["IParameters1*", "int", "void*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "New", slot: 5, base: "IPubKeyEng", ret: "int",
                params: &["IPubKeyEng*", "IParameters1*", "IPubKey**"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IPubKeyUtil",
        base: "IQI",
        declared_in: "AEEIPubKeyUtil.h",
        iid: 0x0103450b,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Parse", slot: 3, base: "IPubKeyUtil", ret: "int",
                params: &["IPubKeyUtil*", "int", "uint8*", "int", "IParameters1*"],
                words: 5,
            },
            MethodInfo {
                name: "Serialize", slot: 4, base: "IPubKeyUtil", ret: "int",
                params: &["IPubKeyUtil*", "int", "IParameters1*", "uint8*", "int", "int*"],
                words: 6,
            },
            MethodInfo {
                name: "AddOIDMapping", slot: 5, base: "IPubKeyUtil", ret: "int",
                params: &["IPubKeyUtil*", "unsigned", "uint32", "int"],
                words: 4,
            },
            MethodInfo {
                name: "LookupOIDMapping", slot: 6, base: "IPubKeyUtil", ret: "int",
                params: &["IPubKeyUtil*", "unsigned", "uint32*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "Hash", slot: 7, base: "IPubKeyUtil", ret: "int",
                params: &["IPubKeyUtil*", "IxOpts*", "unsigned", "uint8*", "int", "uint8*", "int*", "int*"],
                words: 8,
            },
        ],
    },
    IfaceInfo {
        name: "IQI",
        base: "",
        declared_in: "AEEIQI.h",
        iid: 0x01000001,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IQoSBundle",
        base: "IQI",
        declared_in: "AEEIQoSBundle.h",
        iid: 0x01037ad4,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SelectNetworkEx", slot: 3, base: "IQoSBundle", ret: "int",
                params: &["IQoSBundle*", "int", "int16"],
                words: 3,
            },
            MethodInfo {
                name: "Open", slot: 4, base: "IQoSBundle", ret: "int",
                params: &["IQoSBundle*", "IQoSList*", "IQoSList*"],
                words: 3,
            },
            MethodInfo {
                name: "GoActive", slot: 5, base: "IQoSBundle", ret: "int",
                params: &["IQoSBundle*", "IQoSList*"],
                words: 2,
            },
            MethodInfo {
                name: "Close", slot: 6, base: "IQoSBundle", ret: "int",
                params: &["IQoSBundle*", "IQoSList*"],
                words: 2,
            },
            MethodInfo {
                name: "Deactivate", slot: 7, base: "IQoSBundle", ret: "int",
                params: &["IQoSBundle*", "IQoSList*"],
                words: 2,
            },
            MethodInfo {
                name: "AddRetryCB", slot: 8, base: "IQoSBundle", ret: "int",
                params: &["IQoSBundle*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetError", slot: 9, base: "IQoSBundle", ret: "int",
                params: &["IQoSBundle*", "AEEQoSSessionError*"],
                words: 2,
            },
            MethodInfo {
                name: "ExtFunc", slot: 10, base: "IQoSBundle", ret: "int",
                params: &["IQoSBundle*", "AEECLSID", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IQoSFilter",
        base: "IxOpts",
        declared_in: "AEEIQoSFilter.h",
        iid: 0x0102b6cc,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "ExtFunc", slot: 6, base: "IQoSFilter", ret: "int",
                params: &["IQoSFilter*", "int", "uint32"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IQoSFlow",
        base: "IxOpts",
        declared_in: "AEEIQoSFlow.h",
        iid: 0x0102b6cb,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "ExtFunc", slot: 6, base: "IQoSFlow", ret: "int",
                params: &["IQoSFlow*", "int", "uint32"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IQoSList",
        base: "IQI",
        declared_in: "AEEIQoSList.h",
        iid: 0x0102b6ca,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Add", slot: 3, base: "IQoSList", ret: "int",
                params: &["IQoSList*", "IQI*"],
                words: 2,
            },
            MethodInfo {
                name: "Remove", slot: 4, base: "IQoSList", ret: "int",
                params: &["IQoSList*", "IQI*"],
                words: 2,
            },
            MethodInfo {
                name: "Size", slot: 5, base: "IQoSList", ret: "uint32",
                params: &["IQoSList*"],
                words: 1,
            },
            MethodInfo {
                name: "AddAt", slot: 6, base: "IQoSList", ret: "int",
                params: &["IQoSList*", "uint32", "IQI*"],
                words: 3,
            },
            MethodInfo {
                name: "RemoveAt", slot: 7, base: "IQoSList", ret: "int",
                params: &["IQoSList*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetAt", slot: 8, base: "IQoSList", ret: "int",
                params: &["IQoSList*", "uint32", "IQI**"],
                words: 3,
            },
            MethodInfo {
                name: "Find", slot: 9, base: "IQoSList", ret: "int",
                params: &["IQoSList*", "IQI*"],
                words: 2,
            },
            MethodInfo {
                name: "Clear", slot: 10, base: "IQoSList", ret: "void",
                params: &["IQoSList*"],
                words: 1,
            },
            MethodInfo {
                name: "ExtFunc", slot: 11, base: "IQoSList", ret: "int",
                params: &["IQoSList*", "int", "uint32"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IQoSSession",
        base: "IModel",
        declared_in: "AEEIQoSSession.h",
        iid: 0x0103bce1,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddListener", slot: 3, base: "IModel", ret: "int",
                params: &["IModel*", "ModelListener*"],
                words: 2,
            },
            MethodInfo {
                name: "Notify", slot: 4, base: "IModel", ret: "void",
                params: &["IModel*", "ModelEvent*"],
                words: 2,
            },
            MethodInfo {
                name: "SelectNetwork", slot: 5, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "Open", slot: 6, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*", "IQoSSpec*"],
                words: 2,
            },
            MethodInfo {
                name: "Modify", slot: 7, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*", "IQoSSpec*"],
                words: 2,
            },
            MethodInfo {
                name: "Close", slot: 8, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*"],
                words: 1,
            },
            MethodInfo {
                name: "AddRetryCB", slot: 9, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetFlowSpec", slot: 10, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*", "IQoSFlow**", "IQoSFlow**"],
                words: 3,
            },
            MethodInfo {
                name: "GetStatus", slot: 11, base: "IQoSSession", ret: "AEEQoSStatus",
                params: &["IQoSSession*"],
                words: 1,
            },
            MethodInfo {
                name: "GoActive", slot: 12, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*"],
                words: 1,
            },
            MethodInfo {
                name: "SetDormancyTimeout", slot: 13, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*", "uint8"],
                words: 2,
            },
            MethodInfo {
                name: "GetError", slot: 14, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*", "AEEQoSError*"],
                words: 2,
            },
            MethodInfo {
                name: "ExtFunc", slot: 15, base: "IQoSSession", ret: "int",
                params: &["IQoSSession*", "int", "uint32"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IQoSSpec",
        base: "IQI",
        declared_in: "AEEIQoSSpec.h",
        iid: 0x0102b6c6,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SetRxFlowSpec", slot: 3, base: "IQoSSpec", ret: "int",
                params: &["IQoSSpec*", "IQoSList*"],
                words: 2,
            },
            MethodInfo {
                name: "SetTxFlowSpec", slot: 4, base: "IQoSSpec", ret: "int",
                params: &["IQoSSpec*", "IQoSList*"],
                words: 2,
            },
            MethodInfo {
                name: "SetRxFilterSpec", slot: 5, base: "IQoSSpec", ret: "int",
                params: &["IQoSSpec*", "IQoSList*"],
                words: 2,
            },
            MethodInfo {
                name: "SetTxFilterSpec", slot: 6, base: "IQoSSpec", ret: "int",
                params: &["IQoSSpec*", "IQoSList*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRxFlowSpec", slot: 7, base: "IQoSSpec", ret: "int",
                params: &["IQoSSpec*", "IQoSList**"],
                words: 2,
            },
            MethodInfo {
                name: "GetTxFlowSpec", slot: 8, base: "IQoSSpec", ret: "int",
                params: &["IQoSSpec*", "IQoSList**"],
                words: 2,
            },
            MethodInfo {
                name: "GetRxFilterSpec", slot: 9, base: "IQoSSpec", ret: "int",
                params: &["IQoSSpec*", "IQoSList**"],
                words: 2,
            },
            MethodInfo {
                name: "GetTxFilterSpec", slot: 10, base: "IQoSSpec", ret: "int",
                params: &["IQoSSpec*", "IQoSList**"],
                words: 2,
            },
            MethodInfo {
                name: "ExtFunc", slot: 11, base: "IQoSSpec", ret: "int",
                params: &["IQoSSpec*", "int", "uint32"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IRSA",
        base: "IBase",
        declared_in: "AEERSA.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Init", slot: 2, base: "IRSA", ret: "int",
                params: &["IRSA*", "byte*", "int", "byte*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "RSA", slot: 3, base: "IRSA", ret: "int",
                params: &["IRSA*", "byte*", "int", "byte**", "int*", "int"],
                words: 6,
            },
            MethodInfo {
                name: "ModExp", slot: 4, base: "IRSA", ret: "void",
                params: &["IRSA*", "byte*", "int", "byte**", "int*", "uint32*", "AEECallback*"],
                words: 7,
            },
            MethodInfo {
                name: "Encrypt", slot: 5, base: "IRSA", ret: "void",
                params: &["IRSA*", "byte*", "int", "byte*", "int*", "int", "uint32*", "AEECallback*"],
                words: 8,
            },
            MethodInfo {
                name: "Decrypt", slot: 6, base: "IRSA", ret: "void",
                params: &["IRSA*", "byte*", "int", "byte*", "int*", "int", "uint32*", "AEECallback*"],
                words: 8,
            },
            MethodInfo {
                name: "VerifySig", slot: 7, base: "IRSA", ret: "void",
                params: &["IRSA*", "byte*", "int", "byte*", "int", "int", "uint32*", "AEECallback*"],
                words: 8,
            },
        ],
    },
    IfaceInfo {
        name: "IRamCache",
        base: "IRecordStore",
        declared_in: "AEEIRamCache.h",
        iid: 0x0103615f,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Add", slot: 3, base: "IRecordStore", ret: "int",
                params: &["IRecordStore*", "char*", "int", "char*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "Remove", slot: 4, base: "IRecordStore", ret: "int",
                params: &["IRecordStore*", "char*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Find", slot: 5, base: "IRecordStore", ret: "int",
                params: &["IRecordStore*", "char*", "int", "char*", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "SetTTL", slot: 6, base: "IRamCache", ret: "uint32",
                params: &["IRamCache*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "SetMaxSize", slot: 7, base: "IRamCache", ret: "int",
                params: &["IRamCache*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "SetMaxEntries", slot: 8, base: "IRamCache", ret: "int",
                params: &["IRamCache*", "int"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IRawBlockCipher",
        base: "IParameters",
        declared_in: "AEEIRawBlockCipher.h",
        iid: 0x0102d154,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetParam", slot: 3, base: "IParametersRO", ret: "int",
                params: &["IParametersRO*", "int", "void*", "unsigned*"],
                words: 4,
            },
            MethodInfo {
                name: "SetParam", slot: 4, base: "IParameters", ret: "int",
                params: &["IParameters*", "int", "void*", "unsigned"],
                words: 4,
            },
            MethodInfo {
                name: "ProcessBlocks", slot: 5, base: "IRawBlockCipher", ret: "int",
                params: &["IRawBlockCipher*", "byte*", "unsigned", "byte*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IRealloc",
        base: "IQI",
        declared_in: "AEEIRealloc.h",
        iid: 0x0102345e,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "ErrRealloc", slot: 3, base: "IRealloc", ret: "int",
                params: &["IRealloc*", "int", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "ErrReallocName", slot: 4, base: "IRealloc", ret: "int",
                params: &["IRealloc*", "int", "void**", "char*"],
                words: 4,
            },
            MethodInfo {
                name: "ErrReallocNoZI", slot: 5, base: "IRealloc", ret: "int",
                params: &["IRealloc*", "int", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "ErrReallocNameNoZI", slot: 6, base: "IRealloc", ret: "int",
                params: &["IRealloc*", "int", "void**", "char*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IRecordStore",
        base: "IQI",
        declared_in: "AEEIRecordStore.h",
        iid: 0x0103615e,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Add", slot: 3, base: "IRecordStore", ret: "int",
                params: &["IRecordStore*", "char*", "int", "char*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "Remove", slot: 4, base: "IRecordStore", ret: "int",
                params: &["IRecordStore*", "char*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "Find", slot: 5, base: "IRecordStore", ret: "int",
                params: &["IRecordStore*", "char*", "int", "char*", "int*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IRegistry",
        base: "IBase",
        declared_in: "AEERegistry.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "GetHandler", slot: 2, base: "IRegistry", ret: "AEECLSID",
                params: &["IRegistry*", "AEECLSID", "char*"],
                words: 3,
            },
            MethodInfo {
                name: "SetHandler", slot: 3, base: "IRegistry", ret: "int",
                params: &["IRegistry*", "AEECLSID", "char*", "AEECLSID"],
                words: 4,
            },
            MethodInfo {
                name: "Enum", slot: 4, base: "IRegistry", ret: "void",
                params: &["IRegistry*", "PFNREGENUM", "void*", "boolean"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IRemoteControl_Target",
        base: "IQI",
        declared_in: "AEEIRemoteControl_Target.h",
        iid: 0x01062245,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "RegCommandMask", slot: 3, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*", "uint64struct*"],
                words: 2,
            },
            MethodInfo {
                name: "OnCommandNotify", slot: 4, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetCommand", slot: 5, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*", "AEERemoteControl_CommandType*"],
                words: 2,
            },
            MethodInfo {
                name: "SetFocus", slot: 6, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*"],
                words: 1,
            },
            MethodInfo {
                name: "GetSupportedTransports", slot: 7, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*", "AEERemoteControl_Transport*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "RegVendorID", slot: 8, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*", "char*", "AEERemoteControl_Transport"],
                words: 3,
            },
            MethodInfo {
                name: "OnVendorCommandNotify", slot: 9, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "GetVendorCommand", slot: 10, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*", "AEERemoteControl_VendorCommandType*"],
                words: 2,
            },
            MethodInfo {
                name: "ReplyVendorCommand", slot: 11, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*", "AEERemoteControl_VendorCommandType*"],
                words: 2,
            },
            MethodInfo {
                name: "RegExtension", slot: 12, base: "IRemoteControl_Target", ret: "AEEResult",
                params: &["IRemoteControl_Target*", "IQI*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IResourceCtl",
        base: "IQI",
        declared_in: "AEEIResourceCtl.h",
        iid: 0x0101e52e,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Request", slot: 3, base: "IResourceCtl", ret: "int",
                params: &["IResourceCtl*", "uint32", "uint32*"],
                words: 3,
            },
            MethodInfo {
                name: "CanAcquire", slot: 4, base: "IResourceCtl", ret: "int",
                params: &["IResourceCtl*", "uint32", "uint32*"],
                words: 3,
            },
            MethodInfo {
                name: "SetRelinquishCtl", slot: 5, base: "IResourceCtl", ret: "int",
                params: &["IResourceCtl*", "AEECLSID*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "GetStatus", slot: 6, base: "IResourceCtl", ret: "uint32",
                params: &["IResourceCtl*", "uint32*"],
                words: 2,
            },
            MethodInfo {
                name: "OnStatusChangeNotify", slot: 7, base: "IResourceCtl", ret: "int",
                params: &["IResourceCtl*", "uint32", "AEECallback*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IRingerMgr",
        base: "IBase",
        declared_in: "AEERinger.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "RegisterNotify", slot: 2, base: "IRingerMgr", ret: "void",
                params: &["IRingerMgr*", "PFNRINGEREVENT", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "GetNumberFormats", slot: 3, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*"],
                words: 1,
            },
            MethodInfo {
                name: "GetFormats", slot: 4, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*", "AEESoundPlayerFile*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "SetRinger", slot: 5, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*", "AEERingerCatID", "AEERingerID"],
                words: 3,
            },
            MethodInfo {
                name: "EnumCategoryInit", slot: 6, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumNextCategory", slot: 7, base: "IRingerMgr", ret: "boolean",
                params: &["IRingerMgr*", "AEERingerCat*"],
                words: 2,
            },
            MethodInfo {
                name: "EnumRingerInit", slot: 8, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumNextRinger", slot: 9, base: "IRingerMgr", ret: "boolean",
                params: &["IRingerMgr*", "AEERingerInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRingerID", slot: 10, base: "IRingerMgr", ret: "AEERingerID",
                params: &["IRingerMgr*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRingerInfo", slot: 11, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*", "AEERingerID", "AEERingerInfo*"],
                words: 3,
            },
            MethodInfo {
                name: "Create", slot: 12, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*", "AECHAR*", "AEESoundPlayerFile", "IAStream*"],
                words: 4,
            },
            MethodInfo {
                name: "Remove", slot: 13, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*", "AEERingerID"],
                words: 2,
            },
            MethodInfo {
                name: "Play", slot: 14, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*", "AEERingerID", "char*", "IAStream*", "uint32"],
                words: 5,
            },
            MethodInfo {
                name: "Stop", slot: 15, base: "IRingerMgr", ret: "int",
                params: &["IRingerMgr*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IRscPool",
        base: "IQI",
        declared_in: "AEEIRscPool.h",
        iid: 0x010367a7,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Malloc", slot: 3, base: "IRscPool", ret: "void*",
                params: &["IRscPool*", "unsigned"],
                words: 2,
            },
            MethodInfo {
                name: "Free", slot: 4, base: "IRscPool", ret: "void",
                params: &["IRscPool*", "void*"],
                words: 2,
            },
            MethodInfo {
                name: "HoldRsc", slot: 5, base: "IRscPool", ret: "int",
                params: &["IRscPool*", "IBase*"],
                words: 2,
            },
            MethodInfo {
                name: "ReleaseRsc", slot: 6, base: "IRscPool", ret: "uint32",
                params: &["IRscPool*", "IBase*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ISMS",
        base: "IQueryInterface",
        declared_in: "AEESMS.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SendMsg", slot: 3, base: "ISMS", ret: "void",
                params: &["ISMS*", "ISMSMsg*", "AEECallback*", "uint32*"],
                words: 4,
            },
            MethodInfo {
                name: "ReceiveMsg", slot: 4, base: "ISMS", ret: "int",
                params: &["ISMS*", "uint32", "ISMSMsg**"],
                words: 3,
            },
            MethodInfo {
                name: "SetClientStatus", slot: 5, base: "ISMS", ret: "int",
                params: &["ISMS*", "AEESMSClientStatus", "AEESMSType"],
                words: 3,
            },
            MethodInfo {
                name: "GetBytesAvailableForPayload", slot: 6, base: "ISMS", ret: "int",
                params: &["ISMS*", "ISMSMsg*", "int*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetEncodingsAvailableForMOSMS", slot: 7, base: "ISMS", ret: "int",
                params: &["ISMS*", "uint32*", "uint32*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "ISMSBCConfig",
        base: "IQueryInterface",
        declared_in: "AEESMS.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetBCConfig", slot: 3, base: "ISMSBCConfig", ret: "void",
                params: &["ISMSBCConfig*", "AEESMSMode", "AEECallback*", "AEESMSBCConfig*", "uint32*"],
                words: 5,
            },
            MethodInfo {
                name: "GetBCPref", slot: 4, base: "ISMSBCConfig", ret: "void",
                params: &["ISMSBCConfig*", "AEESMSMode", "AEECallback*", "AEESMSBCPref*", "uint32*"],
                words: 5,
            },
            MethodInfo {
                name: "SetBCPref", slot: 5, base: "ISMSBCConfig", ret: "void",
                params: &["ISMSBCConfig*", "AEESMSMode", "AEESMSBCPref", "AEECallback*", "uint32*"],
                words: 5,
            },
            MethodInfo {
                name: "GetServiceIDs", slot: 6, base: "ISMSBCConfig", ret: "void",
                params: &["ISMSBCConfig*", "AEESMSMode", "AEECallback*", "uint32*", "AEESMSBCSrvID**", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "GetServiceOpts", slot: 7, base: "ISMSBCConfig", ret: "void",
                params: &["ISMSBCConfig*", "AEESMSMode", "AEESMSBCSrvID", "AEECallback*", "ISMSBCSrvOpts**", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "SetServiceOpts", slot: 8, base: "ISMSBCConfig", ret: "void",
                params: &["ISMSBCConfig*", "AEESMSMode", "ISMSBCSrvOpts*", "AEECallback*", "uint32*"],
                words: 5,
            },
            MethodInfo {
                name: "GetAllServiceOpts", slot: 9, base: "ISMSBCConfig", ret: "void",
                params: &["ISMSBCConfig*", "AEESMSMode", "AEECallback*", "uint32*", "ISMSBCSrvOpts***", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "DeleteService", slot: 10, base: "ISMSBCConfig", ret: "void",
                params: &["ISMSBCConfig*", "AEESMSMode", "AEESMSBCSrvID", "AEECallback*", "uint32*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "ISMSStorage",
        base: "IQueryInterface",
        declared_in: "AEESMS.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetStorageStatus", slot: 3, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "AEESMSTag", "AEECallback*", "AEESMSStorageStatus*", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "StoreMsg", slot: 4, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "ISMSMsg*", "AEECallback*", "uint32*", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "ReadMsg", slot: 5, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "uint32", "AEECallback*", "ISMSMsg**", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "Delete", slot: 6, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "AEESMSTag", "uint32", "AEECallback*", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "EnumMsgInit", slot: 7, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "AEESMSTag", "AEECallback*", "uint32*"],
                words: 5,
            },
            MethodInfo {
                name: "EnumNextMsg", slot: 8, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "AEECallback*", "uint32*", "ISMSMsg**", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "UpdateMsg", slot: 9, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "uint32", "ISMSMsg*", "AEECallback*", "uint32*"],
                words: 6,
            },
        ],
    },
    IfaceInfo {
        name: "ISMSStorage2",
        base: "ISMSStorage",
        declared_in: "AEESMS.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetStorageStatus", slot: 3, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "AEESMSTag", "AEECallback*", "AEESMSStorageStatus*", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "StoreMsg", slot: 4, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "ISMSMsg*", "AEECallback*", "uint32*", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "ReadMsg", slot: 5, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "uint32", "AEECallback*", "ISMSMsg**", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "Delete", slot: 6, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "AEESMSTag", "uint32", "AEECallback*", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "EnumMsgInit", slot: 7, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "AEESMSTag", "AEECallback*", "uint32*"],
                words: 5,
            },
            MethodInfo {
                name: "EnumNextMsg", slot: 8, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "AEECallback*", "uint32*", "ISMSMsg**", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "UpdateMsg", slot: 9, base: "ISMSStorage", ret: "void",
                params: &["ISMSStorage*", "AEESMSStorageType", "uint32", "ISMSMsg*", "AEECallback*", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "AutoStoreMsg", slot: 10, base: "ISMSStorage2", ret: "void",
                params: &["ISMSStorage2*", "AEESMSClass", "AEESMSStorageType", "AEECallback*", "uint32*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "ISSL",
        base: "IWebOpts",
        declared_in: "AEESSL.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "SetSocket", slot: 6, base: "ISSL", ret: "void",
                params: &["ISSL*", "ISocket*"],
                words: 2,
            },
            MethodInfo {
                name: "Negotiate", slot: 7, base: "ISSL", ret: "void",
                params: &["ISSL*", "SSLInfo*", "IX509Chain**", "AEECallback*", "..."],
                words: 5,
            },
            MethodInfo {
                name: "NegotiateV", slot: 8, base: "ISSL", ret: "void",
                params: &["ISSL*", "SSLInfo*", "IX509Chain**", "AEECallback*", "WebOpt*"],
                words: 5,
            },
            MethodInfo {
                name: "Renegotiate", slot: 9, base: "ISSL", ret: "void",
                params: &["ISSL*", "SSLInfo*", "IX509Chain**", "AEECallback*", "..."],
                words: 5,
            },
            MethodInfo {
                name: "RenegotiateV", slot: 10, base: "ISSL", ret: "void",
                params: &["ISSL*", "SSLInfo*", "IX509Chain**", "AEECallback*", "WebOpt*"],
                words: 5,
            },
            MethodInfo {
                name: "Close", slot: 11, base: "ISSL", ret: "void",
                params: &["ISSL*", "SSLInfo*", "AEECallback*"],
                words: 3,
            },
            MethodInfo {
                name: "SetPort", slot: 12, base: "ISSL", ret: "void",
                params: &["ISSL*", "IPort*", "void*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IShell",
        base: "IBase",
        declared_in: "AEEShell.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "CreateInstance", slot: 2, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "QueryClass", slot: 3, base: "IShell", ret: "boolean",
                params: &["IShell*", "AEECLSID", "AEEAppInfo*"],
                words: 3,
            },
            MethodInfo {
                name: "GetDeviceInfo", slot: 4, base: "IShell", ret: "void",
                params: &["IShell*", "AEEDeviceInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "StartApplet", slot: 5, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "uint16", "char*"],
                words: 4,
            },
            MethodInfo {
                name: "CloseApplet", slot: 6, base: "IShell", ret: "int",
                params: &["IShell*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "CanStartApplet", slot: 7, base: "IShell", ret: "boolean",
                params: &["IShell*", "AEECLSID"],
                words: 2,
            },
            MethodInfo {
                name: "ActiveApplet", slot: 8, base: "IShell", ret: "AEECLSID",
                params: &["IShell*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumAppletInit", slot: 9, base: "IShell", ret: "void",
                params: &["IShell*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumNextApplet", slot: 10, base: "IShell", ret: "AEECLSID",
                params: &["IShell*", "AEEAppInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "SetTimer", slot: 11, base: "IShell", ret: "int",
                params: &["IShell*", "int32", "PFNNOTIFY", "void*"],
                words: 4,
            },
            MethodInfo {
                name: "CancelTimer", slot: 12, base: "IShell", ret: "int",
                params: &["IShell*", "PFNNOTIFY", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "GetTimerExpiration", slot: 13, base: "IShell", ret: "uint32",
                params: &["IShell*", "PFNNOTIFY", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "CreateDialog", slot: 14, base: "IShell", ret: "int",
                params: &["IShell*", "char*", "uint16", "DialogInfo*"],
                words: 4,
            },
            MethodInfo {
                name: "GetActiveDialog", slot: 15, base: "IShell", ret: "IDialog*",
                params: &["IShell*"],
                words: 1,
            },
            MethodInfo {
                name: "EndDialog", slot: 16, base: "IShell", ret: "int",
                params: &["IShell*"],
                words: 1,
            },
            MethodInfo {
                name: "LoadResString", slot: 17, base: "IShell", ret: "int",
                params: &["IShell*", "char*", "uint16", "AECHAR*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "LoadResData", slot: 18, base: "IShell", ret: "void*",
                params: &["IShell*", "char*", "uint16", "ResType"],
                words: 4,
            },
            MethodInfo {
                name: "LoadResObject", slot: 19, base: "IShell", ret: "IBase*",
                params: &["IShell*", "char*", "uint16", "AEECLSID"],
                words: 4,
            },
            MethodInfo {
                name: "FreeResData", slot: 20, base: "IShell", ret: "void",
                params: &["IShell*", "void*"],
                words: 2,
            },
            MethodInfo {
                name: "SendEvent", slot: 21, base: "IShell", ret: "boolean",
                params: &["IShell*", "uint16", "AEECLSID", "AEEEvent", "uint16", "uint32"],
                words: 6,
            },
            MethodInfo {
                name: "Beep", slot: 22, base: "IShell", ret: "boolean",
                params: &["IShell*", "BeepType", "boolean"],
                words: 3,
            },
            MethodInfo {
                name: "GetPrefs", slot: 23, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "uint16", "void*", "uint16"],
                words: 5,
            },
            MethodInfo {
                name: "SetPrefs", slot: 24, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "uint16", "void*", "uint16"],
                words: 5,
            },
            MethodInfo {
                name: "GetItemStyle", slot: 25, base: "IShell", ret: "void",
                params: &["IShell*", "AEEItemType", "AEEItemStyle*", "AEEItemStyle*"],
                words: 4,
            },
            MethodInfo {
                name: "Prompt", slot: 26, base: "IShell", ret: "boolean",
                params: &["IShell*", "AEEPromptInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "MessageBox", slot: 27, base: "IShell", ret: "boolean",
                params: &["IShell*", "char*", "uint16", "uint16"],
                words: 4,
            },
            MethodInfo {
                name: "MessageBoxText", slot: 28, base: "IShell", ret: "boolean",
                params: &["IShell*", "AECHAR*", "AECHAR*"],
                words: 3,
            },
            MethodInfo {
                name: "SetAlarm", slot: 29, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "CancelAlarm", slot: 30, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "AlarmsActive", slot: 31, base: "IShell", ret: "boolean",
                params: &["IShell*"],
                words: 1,
            },
            MethodInfo {
                name: "GetHandler", slot: 32, base: "IShell", ret: "AEECLSID",
                params: &["IShell*", "AEECLSID", "char*"],
                words: 3,
            },
            MethodInfo {
                name: "RegisterHandler", slot: 33, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "char*", "AEECLSID"],
                words: 4,
            },
            MethodInfo {
                name: "RegisterNotify", slot: 34, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "AEECLSID", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Notify", slot: 35, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "uint32", "void*"],
                words: 4,
            },
            MethodInfo {
                name: "Resume", slot: 36, base: "IShell", ret: "void",
                params: &["IShell*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "ForceExit", slot: 37, base: "IShell", ret: "boolean",
                params: &["IShell*"],
                words: 1,
            },
            MethodInfo {
                name: "GetPosition", slot: 38, base: "IShell", ret: "int",
                params: &["IShell*", "AEEPosAccuracy", "PFNPOSITIONCB", "void*"],
                words: 4,
            },
            MethodInfo {
                name: "CheckPrivLevel", slot: 39, base: "IShell", ret: "boolean",
                params: &["IShell*", "AEECLSID", "boolean"],
                words: 3,
            },
            MethodInfo {
                name: "IsValidResource", slot: 40, base: "IShell", ret: "boolean",
                params: &["IShell*", "char*", "uint16", "ResType", "AEECLSID"],
                words: 5,
            },
            MethodInfo {
                name: "LoadResDataEx", slot: 41, base: "IShell", ret: "void*",
                params: &["IShell*", "char*", "uint16", "ResType", "void*", "uint32*"],
                words: 6,
            },
            MethodInfo {
                name: "RegisterSystemCallback", slot: 42, base: "IShell", ret: "void",
                params: &["IShell*", "AEECallback*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "DetectType", slot: 43, base: "IShell", ret: "int",
                params: &["IShell*", "void*", "uint32*", "char*", "char**"],
                words: 5,
            },
            MethodInfo {
                name: "GetDeviceInfoEx", slot: 44, base: "IShell", ret: "int",
                params: &["IShell*", "AEEDeviceItem", "void*", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetClassItemID", slot: 45, base: "IShell", ret: "uint32",
                params: &["IShell*", "AEECLSID"],
                words: 2,
            },
            MethodInfo {
                name: "Obsolete", slot: 46, base: "IShell", ret: "int",
                params: &["IShell*"],
                words: 1,
            },
            MethodInfo {
                name: "GetProperty", slot: 47, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "uint16", "void*", "uint16*"],
                words: 5,
            },
            MethodInfo {
                name: "SetProperty", slot: 48, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "uint16", "void*", "uint16"],
                words: 5,
            },
            MethodInfo {
                name: "RegisterEvent", slot: 49, base: "IShell", ret: "AEEEvent",
                params: &["IShell*", "char*", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "Reset", slot: 50, base: "IShell", ret: "int",
                params: &["IShell*", "AEEResetType"],
                words: 2,
            },
            MethodInfo {
                name: "AppIsInGroup", slot: 51, base: "IShell", ret: "int",
                params: &["IShell*", "AEECLSID", "AEECLSID"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "ISignal",
        base: "IQI",
        declared_in: "AEEISignal.h",
        iid: 0x010285f5,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Set", slot: 3, base: "ISignal", ret: "int",
                params: &["ISignal*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "ISignalBus",
        base: "IQI",
        declared_in: "AEEISignalBus.h",
        iid: 0x0103d14b,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Strobe", slot: 3, base: "ISignalBus", ret: "int",
                params: &["ISignalBus*"],
                words: 1,
            },
            MethodInfo {
                name: "Set", slot: 4, base: "ISignalBus", ret: "int",
                params: &["ISignalBus*"],
                words: 1,
            },
            MethodInfo {
                name: "Clear", slot: 5, base: "ISignalBus", ret: "int",
                params: &["ISignalBus*"],
                words: 1,
            },
            MethodInfo {
                name: "OnEnable", slot: 6, base: "ISignalBus", ret: "int",
                params: &["ISignalBus*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "Add", slot: 7, base: "ISignalBus", ret: "int",
                params: &["ISignalBus*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "Remove", slot: 8, base: "ISignalBus", ret: "int",
                params: &["ISignalBus*", "ISignal*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ISignalCBFactory",
        base: "IQI",
        declared_in: "AEEISignalCBFactory.h",
        iid: 0x01043541,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "CreateSignal", slot: 3, base: "ISignalCBFactory", ret: "int",
                params: &["ISignalCBFactory*", "fnptr", "void*", "ISignal**", "ISignalCtl**"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "ISignalCtl",
        base: "ISignal",
        declared_in: "AEEISignalCtl.h",
        iid: 0x01041079,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Set", slot: 3, base: "ISignal", ret: "int",
                params: &["ISignal*"],
                words: 1,
            },
            MethodInfo {
                name: "Detach", slot: 4, base: "ISignalCtl", ret: "int",
                params: &["ISignalCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "Enable", slot: 5, base: "ISignalCtl", ret: "int",
                params: &["ISignalCtl*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "ISingleton",
        base: "IQI",
        declared_in: "AEEISingleton.h",
        iid: 0x0101ed1f,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "CreateInstance", slot: 3, base: "ISingleton", ret: "int",
                params: &["ISingleton*", "AEECLSID", "void**", "SingletonCIFunc*", "void*"],
                words: 5,
            },
            MethodInfo {
                name: "Ref", slot: 4, base: "ISingleton", ret: "uint32",
                params: &["ISingleton*", "AEECLSID", "uint32*", "int"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "ISockPort",
        base: "IPort",
        declared_in: "AEEISockPort.h",
        iid: 0x01039ba8,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetLastError", slot: 5, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Write", slot: 6, base: "IPort", ret: "int32",
                params: &["IPort*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Writeable", slot: 7, base: "IPort", ret: "void",
                params: &["IPort*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "IOCtl", slot: 8, base: "IPort", ret: "int",
                params: &["IPort*", "int", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Close", slot: 9, base: "IPort", ret: "int",
                params: &["IPort*"],
                words: 1,
            },
            MethodInfo {
                name: "Open", slot: 10, base: "IPort", ret: "int",
                params: &["IPort*", "char*"],
                words: 2,
            },
            MethodInfo {
                name: "OpenEx", slot: 11, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "uint16", "AEESockType", "int"],
                words: 4,
            },
            MethodInfo {
                name: "SelectNetwork", slot: 12, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "Bind", slot: 13, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "void*"],
                words: 2,
            },
            MethodInfo {
                name: "Listen", slot: 14, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "Accept", slot: 15, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "ISockPort**pp##"],
                words: 2,
            },
            MethodInfo {
                name: "Connect", slot: 16, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "void*"],
                words: 2,
            },
            MethodInfo {
                name: "WriteV", slot: 17, base: "ISockPort", ret: "int32",
                params: &["ISockPort*", "SockIOBlock iov", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "ReadV", slot: 18, base: "ISockPort", ret: "int32",
                params: &["ISockPort*", "SockIOBlock iov", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "SendTo", slot: 19, base: "ISockPort", ret: "int32",
                params: &["ISockPort*", "char*", "int32", "uint32", "void*"],
                words: 5,
            },
            MethodInfo {
                name: "RecvFrom", slot: 20, base: "ISockPort", ret: "int32",
                params: &["ISockPort*", "char*", "int32", "uint32", "void*", "int*"],
                words: 6,
            },
            MethodInfo {
                name: "GetSockName", slot: 21, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "void*", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "GetPeerName", slot: 22, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "void*", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 23, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "int", "int", "void*", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "SetOpt", slot: 24, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "int", "int", "void*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "Shutdown", slot: 25, base: "ISockPort", ret: "int",
                params: &["ISockPort*", "int"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ISocket",
        base: "IAStream",
        declared_in: "AEENet.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Readable", slot: 2, base: "IAStream", ret: "void",
                params: &["IAStream*", "fnptr", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "IAStream", ret: "int32",
                params: &["IAStream*", "void*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Cancel", slot: 4, base: "IAStream", ret: "void",
                params: &["IAStream*", "fnptr", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "GetPeerName", slot: 5, base: "ISocket", ret: "int",
                params: &["ISocket*", "INAddr*", "INPort*"],
                words: 3,
            },
            MethodInfo {
                name: "GetLastError", slot: 6, base: "ISocket", ret: "int",
                params: &["ISocket*"],
                words: 1,
            },
            MethodInfo {
                name: "Connect", slot: 7, base: "ISocket", ret: "int",
                params: &["ISocket*", "INAddr", "INPort", "PFNCONNECTCB", "void*"],
                words: 5,
            },
            MethodInfo {
                name: "Bind", slot: 8, base: "ISocket", ret: "int",
                params: &["ISocket*", "INAddr", "INPort"],
                words: 3,
            },
            MethodInfo {
                name: "Write", slot: 9, base: "ISocket", ret: "int32",
                params: &["ISocket*", "byte*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "WriteV", slot: 10, base: "ISocket", ret: "int32",
                params: &["ISocket*", "SockIOBlock iov", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "ReadV", slot: 11, base: "ISocket", ret: "int32",
                params: &["ISocket*", "SockIOBlock iov", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "SendTo", slot: 12, base: "ISocket", ret: "int32",
                params: &["ISocket*", "byte*", "uint16", "uint16", "INAddr", "uint16"],
                words: 6,
            },
            MethodInfo {
                name: "RecvFrom", slot: 13, base: "ISocket", ret: "int32",
                params: &["ISocket*", "byte*", "uint16", "uint16", "INAddr*", "INPort*"],
                words: 6,
            },
            MethodInfo {
                name: "Writeable", slot: 14, base: "ISocket", ret: "void",
                params: &["ISocket*", "PFNNOTIFY", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "IOCtl", slot: 15, base: "ISocket", ret: "int",
                params: &["ISocket*", "int", "uint32"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "ISound",
        base: "IBase",
        declared_in: "AEEISound.h",
        iid: 0x01001056,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "RegisterNotify", slot: 2, base: "ISound", ret: "void",
                params: &["ISound*", "PFNSOUNDSTATUS", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Set", slot: 3, base: "ISound", ret: "int",
                params: &["ISound*", "AEESoundInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "Get", slot: 4, base: "ISound", ret: "void",
                params: &["ISound*", "AEESoundInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "SetDevice", slot: 5, base: "ISound", ret: "void",
                params: &["ISound*"],
                words: 1,
            },
            MethodInfo {
                name: "PlayTone", slot: 6, base: "ISound", ret: "void",
                params: &["ISound*", "AEESoundToneData"],
                words: 2,
            },
            MethodInfo {
                name: "PlayToneList", slot: 7, base: "ISound", ret: "void",
                params: &["ISound*", "AEESoundToneData*", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "PlayFreqTone", slot: 8, base: "ISound", ret: "void",
                params: &["ISound*", "uint16", "uint16", "uint16"],
                words: 4,
            },
            MethodInfo {
                name: "StopTone", slot: 9, base: "ISound", ret: "void",
                params: &["ISound*"],
                words: 1,
            },
            MethodInfo {
                name: "Vibrate", slot: 10, base: "ISound", ret: "void",
                params: &["ISound*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "StopVibrate", slot: 11, base: "ISound", ret: "void",
                params: &["ISound*"],
                words: 1,
            },
            MethodInfo {
                name: "SetVolume", slot: 12, base: "ISound", ret: "void",
                params: &["ISound*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "GetVolume", slot: 13, base: "ISound", ret: "void",
                params: &["ISound*"],
                words: 1,
            },
            MethodInfo {
                name: "GetResourceCtl", slot: 14, base: "ISound", ret: "int",
                params: &["ISound*", "IResourceCtl**"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ISoundPlayer",
        base: "IBase",
        declared_in: "AEESoundPlayer.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "RegisterNotify", slot: 2, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*", "PFNSOUNDPLAYERSTATUS", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Set", slot: 3, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*", "AEESoundPlayerInput", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Play", slot: 4, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*"],
                words: 1,
            },
            MethodInfo {
                name: "Stop", slot: 5, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*"],
                words: 1,
            },
            MethodInfo {
                name: "Rewind", slot: 6, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "FastForward", slot: 7, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "Pause", slot: 8, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*"],
                words: 1,
            },
            MethodInfo {
                name: "Resume", slot: 9, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*"],
                words: 1,
            },
            MethodInfo {
                name: "SetTempo", slot: 10, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "SetTune", slot: 11, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*", "int8"],
                words: 2,
            },
            MethodInfo {
                name: "SetVolume", slot: 12, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "GetVolume", slot: 13, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*"],
                words: 1,
            },
            MethodInfo {
                name: "GetTotalTime", slot: 14, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*"],
                words: 1,
            },
            MethodInfo {
                name: "SetSoundDevice", slot: 15, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*", "AEESoundDevice", "AEESoundMuteCtl", "AEESoundMuteCtl"],
                words: 4,
            },
            MethodInfo {
                name: "SetStream", slot: 16, base: "ISoundPlayer", ret: "void",
                params: &["ISoundPlayer*", "IAStream*"],
                words: 2,
            },
            MethodInfo {
                name: "SetInfo", slot: 17, base: "ISoundPlayer", ret: "int",
                params: &["ISoundPlayer*", "AEESoundPlayerInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "GetInfo", slot: 18, base: "ISoundPlayer", ret: "int",
                params: &["ISoundPlayer*", "AEESoundPlayerInfo*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ISource",
        base: "IQI",
        declared_in: "AEEISource.h",
        iid: 0x01001012,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "ISource", ret: "int32",
                params: &["ISource*", "char*", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "Readable", slot: 4, base: "ISource", ret: "void",
                params: &["ISource*", "AEECallback*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ISourceUtil",
        base: "IQueryInterface",
        declared_in: "AEESource.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetLineFromSource", slot: 3, base: "ISourceUtil", ret: "int32",
                params: &["ISourceUtil*", "ISource*", "int32", "IGetLine**"],
                words: 4,
            },
            MethodInfo {
                name: "SourceFromSocket", slot: 4, base: "ISourceUtil", ret: "int32",
                params: &["ISourceUtil*", "ISocket*", "ISource**"],
                words: 3,
            },
            MethodInfo {
                name: "SourceFromMemory", slot: 5, base: "ISourceUtil", ret: "int32",
                params: &["ISourceUtil*", "void*", "int32", "PFNNOTIFY", "void*", "ISource**"],
                words: 6,
            },
            MethodInfo {
                name: "SourceFromAStream", slot: 6, base: "ISourceUtil", ret: "int32",
                params: &["ISourceUtil*", "IAStream*", "ISource**"],
                words: 3,
            },
            MethodInfo {
                name: "AStreamFromSource", slot: 7, base: "ISourceUtil", ret: "int32",
                params: &["ISourceUtil*", "ISource*", "IAStream**"],
                words: 3,
            },
            MethodInfo {
                name: "PeekFromMemory", slot: 8, base: "ISourceUtil", ret: "int32",
                params: &["ISourceUtil*", "void*", "int32", "PFNNOTIFY", "void*", "IPeek**"],
                words: 6,
            },
            MethodInfo {
                name: "PeekFromSource", slot: 9, base: "ISourceUtil", ret: "int32",
                params: &["ISourceUtil*", "ISource*", "int32", "IPeek**"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "ISprite",
        base: "IQueryInterface",
        declared_in: "AEESprite.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "SetTransformTable", slot: 3, base: "ISprite", ret: "int",
                params: &["ISprite*", "AEETransformMatrix*"],
                words: 2,
            },
            MethodInfo {
                name: "SetSpriteBuffer", slot: 4, base: "ISprite", ret: "int",
                params: &["ISprite*", "uint8", "IBitmap*"],
                words: 3,
            },
            MethodInfo {
                name: "SetTileBuffer", slot: 5, base: "ISprite", ret: "int",
                params: &["ISprite*", "uint8", "IBitmap*"],
                words: 3,
            },
            MethodInfo {
                name: "SetDestination", slot: 6, base: "ISprite", ret: "int",
                params: &["ISprite*", "IBitmap*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawSprites", slot: 7, base: "ISprite", ret: "int",
                params: &["ISprite*", "AEESpriteCmd*"],
                words: 2,
            },
            MethodInfo {
                name: "DrawTiles", slot: 8, base: "ISprite", ret: "int",
                params: &["ISprite*", "AEETileMap*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IStatic",
        base: "IBase",
        declared_in: "AEEShell.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IControl", ret: "boolean",
                params: &["IControl*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Redraw", slot: 3, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetActive", slot: 4, base: "IControl", ret: "void",
                params: &["IControl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "IsActive", slot: 5, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetRect", slot: 6, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRect", slot: 7, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "SetProperties", slot: 8, base: "IControl", ret: "void",
                params: &["IControl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 9, base: "IControl", ret: "uint32",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 10, base: "IControl", ret: "void",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetText", slot: 11, base: "IStatic", ret: "boolean",
                params: &["IStatic*", "AECHAR*", "AECHAR*", "AEEFont", "AEEFont"],
                words: 5,
            },
            MethodInfo {
                name: "SetFont", slot: 12, base: "IStatic", ret: "void",
                params: &["IStatic*", "AEEFont", "AEEFont"],
                words: 3,
            },
            MethodInfo {
                name: "SetTextEx", slot: 13, base: "IStatic", ret: "boolean",
                params: &["IStatic*", "byte*", "IAStream*", "boolean"],
                words: 4,
            },
            MethodInfo {
                name: "GoToLine", slot: 14, base: "IStatic", ret: "int",
                params: &["IStatic*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "SizeToFit", slot: 15, base: "IStatic", ret: "void",
                params: &["IStatic*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "IsScrollable", slot: 16, base: "IStatic", ret: "boolean",
                params: &["IStatic*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "ISuppsTrans",
        base: "IQueryInterface",
        declared_in: "AEETelephone.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "ParseString", slot: 3, base: "ISuppsTrans", ret: "int",
                params: &["ISuppsTrans*", "char*", "AEETSuppsMessage*", "uint32*"],
                words: 4,
            },
            MethodInfo {
                name: "GetState", slot: 4, base: "ISuppsTrans", ret: "AEETSuppsTransState",
                params: &["ISuppsTrans*"],
                words: 1,
            },
            MethodInfo {
                name: "LoadMessage", slot: 5, base: "ISuppsTrans", ret: "int",
                params: &["ISuppsTrans*", "AEETSuppsMessage*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Run", slot: 6, base: "ISuppsTrans", ret: "void",
                params: &["ISuppsTrans*", "AEECallback*"],
                words: 2,
            },
            MethodInfo {
                name: "GetResult", slot: 7, base: "ISuppsTrans", ret: "int",
                params: &["ISuppsTrans*", "AEETSuppsStatus**"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ITAPI",
        base: "IBase",
        declared_in: "AEETAPI.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "SetMask", slot: 2, base: "INotifier", ret: "void",
                params: &["INotifier*", "uint32*"],
                words: 2,
            },
            MethodInfo {
                name: "GetStatus", slot: 3, base: "ITAPI", ret: "int",
                params: &["ITAPI*", "TAPIStatus*"],
                words: 2,
            },
            MethodInfo {
                name: "MakeVoiceCall", slot: 4, base: "ITAPI", ret: "int",
                params: &["ITAPI*", "char*", "AEECLSID"],
                words: 3,
            },
            MethodInfo {
                name: "ExtractSMSText", slot: 5, base: "ITAPI", ret: "AEESMSTextMsg*",
                params: &["ITAPI*", "AEESMSMsg*"],
                words: 2,
            },
            MethodInfo {
                name: "GetCallerID", slot: 6, base: "ITAPI", ret: "boolean",
                params: &["ITAPI*", "AECHAR*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "OnCallStatus", slot: 7, base: "ITAPI", ret: "int",
                params: &["ITAPI*", "PFNNOTIFY", "void*", "uint32", "uint16"],
                words: 5,
            },
            MethodInfo {
                name: "IsVoiceCall", slot: 8, base: "ITAPI", ret: "boolean",
                params: &["ITAPI*"],
                words: 1,
            },
            MethodInfo {
                name: "IsDataSupported", slot: 9, base: "ITAPI", ret: "boolean",
                params: &["ITAPI*"],
                words: 1,
            },
            MethodInfo {
                name: "SendSMS", slot: 10, base: "ITAPI", ret: "int",
                params: &["ITAPI*", "char*", "char*", "AEECLSID", "PFNSMSSTATUS", "void*"],
                words: 6,
            },
        ],
    },
    IfaceInfo {
        name: "ITelephone",
        base: "IQueryInterface",
        declared_in: "AEETelephone.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetPhoneInfo", slot: 3, base: "ITelephone", ret: "int",
                params: &["ITelephone*", "AEETPhInfo*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "GetCallInfo", slot: 4, base: "ITelephone", ret: "int",
                params: &["ITelephone*", "AEECallDesc", "AEETCallInfo*", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "GetServingSystemInfo", slot: 5, base: "ITelephone", ret: "int",
                params: &["ITelephone*", "AEETSSInfo*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Get", slot: 6, base: "ITelephone", ret: "int",
                params: &["ITelephone*", "uint32", "void*", "uint32", "AEECallback*", "void*", "uint32"],
                words: 7,
            },
        ],
    },
    IfaceInfo {
        name: "ITextCtl",
        base: "IBase",
        declared_in: "AEEText.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IControl", ret: "boolean",
                params: &["IControl*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Redraw", slot: 3, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetActive", slot: 4, base: "IControl", ret: "void",
                params: &["IControl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "IsActive", slot: 5, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetRect", slot: 6, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRect", slot: 7, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "SetProperties", slot: 8, base: "IControl", ret: "void",
                params: &["IControl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 9, base: "IControl", ret: "uint32",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 10, base: "IControl", ret: "void",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetTitle", slot: 11, base: "ITextCtl", ret: "boolean",
                params: &["ITextCtl*", "char*", "uint16", "AECHAR*"],
                words: 4,
            },
            MethodInfo {
                name: "SetText", slot: 12, base: "ITextCtl", ret: "boolean",
                params: &["ITextCtl*", "AECHAR*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "GetText", slot: 13, base: "ITextCtl", ret: "boolean",
                params: &["ITextCtl*", "AECHAR*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "GetTextPtr", slot: 14, base: "ITextCtl", ret: "AECHAR*",
                params: &["ITextCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "EnableCommand", slot: 15, base: "ITextCtl", ret: "void",
                params: &["ITextCtl*", "boolean", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "SetMaxSize", slot: 16, base: "ITextCtl", ret: "void",
                params: &["ITextCtl*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "SetSoftKeyMenu", slot: 17, base: "ITextCtl", ret: "void",
                params: &["ITextCtl*", "IMenuCtl*"],
                words: 2,
            },
            MethodInfo {
                name: "SetInputMode", slot: 18, base: "ITextCtl", ret: "AEETextInputMode",
                params: &["ITextCtl*", "AEETextInputMode"],
                words: 2,
            },
            MethodInfo {
                name: "GetCursorPos", slot: 19, base: "ITextCtl", ret: "int32",
                params: &["ITextCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetCursorPos", slot: 20, base: "ITextCtl", ret: "void",
                params: &["ITextCtl*", "int32"],
                words: 2,
            },
            MethodInfo {
                name: "GetInputMode", slot: 21, base: "ITextCtl", ret: "AEETextInputMode",
                params: &["ITextCtl*", "AEETextInputModeInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "EnumModeInit", slot: 22, base: "ITextCtl", ret: "void",
                params: &["ITextCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumNextMode", slot: 23, base: "ITextCtl", ret: "AEETextInputMode",
                params: &["ITextCtl*", "AEETextInputModeInfo*"],
                words: 2,
            },
            MethodInfo {
                name: "SetSelection", slot: 24, base: "ITextCtl", ret: "void",
                params: &["ITextCtl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetSelection", slot: 25, base: "ITextCtl", ret: "uint32",
                params: &["ITextCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetPropertiesEx", slot: 26, base: "ITextCtl", ret: "void",
                params: &["ITextCtl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetPropertiesEx", slot: 27, base: "ITextCtl", ret: "uint32",
                params: &["ITextCtl*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IThread",
        base: "IRscPool",
        declared_in: "AEEThread.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Malloc", slot: 3, base: "IRscPool", ret: "void*",
                params: &["IRscPool*", "unsigned"],
                words: 2,
            },
            MethodInfo {
                name: "Free", slot: 4, base: "IRscPool", ret: "void",
                params: &["IRscPool*", "void*"],
                words: 2,
            },
            MethodInfo {
                name: "HoldRsc", slot: 5, base: "IRscPool", ret: "int",
                params: &["IRscPool*", "IBase*"],
                words: 2,
            },
            MethodInfo {
                name: "ReleaseRsc", slot: 6, base: "IRscPool", ret: "uint32",
                params: &["IRscPool*", "IBase*"],
                words: 2,
            },
            MethodInfo {
                name: "Start", slot: 7, base: "IThread", ret: "int",
                params: &["IThread*", "int", "PFNTHREAD", "void*"],
                words: 4,
            },
            MethodInfo {
                name: "Exit", slot: 8, base: "IThread", ret: "int",
                params: &["IThread*", "int"],
                words: 2,
            },
            MethodInfo {
                name: "Join", slot: 9, base: "IThread", ret: "void",
                params: &["IThread*", "AEECallback*", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "Suspend", slot: 10, base: "IThread", ret: "void",
                params: &["IThread*"],
                words: 1,
            },
            MethodInfo {
                name: "GetResumeCBK", slot: 11, base: "IThread", ret: "AEECallback*",
                params: &["IThread*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "ITimeCtl",
        base: "IBase",
        declared_in: "AEETime.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "HandleEvent", slot: 2, base: "IControl", ret: "boolean",
                params: &["IControl*", "AEEEvent", "uint16", "uint32"],
                words: 4,
            },
            MethodInfo {
                name: "Redraw", slot: 3, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetActive", slot: 4, base: "IControl", ret: "void",
                params: &["IControl*", "boolean"],
                words: 2,
            },
            MethodInfo {
                name: "IsActive", slot: 5, base: "IControl", ret: "boolean",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetRect", slot: 6, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "GetRect", slot: 7, base: "IControl", ret: "void",
                params: &["IControl*", "AEERect*"],
                words: 2,
            },
            MethodInfo {
                name: "SetProperties", slot: 8, base: "IControl", ret: "void",
                params: &["IControl*", "uint32"],
                words: 2,
            },
            MethodInfo {
                name: "GetProperties", slot: 9, base: "IControl", ret: "uint32",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "Reset", slot: 10, base: "IControl", ret: "void",
                params: &["IControl*"],
                words: 1,
            },
            MethodInfo {
                name: "SetTime", slot: 11, base: "ITimeCtl", ret: "boolean",
                params: &["ITimeCtl*", "int32", "boolean"],
                words: 3,
            },
            MethodInfo {
                name: "GetTime", slot: 12, base: "ITimeCtl", ret: "int32",
                params: &["ITimeCtl*"],
                words: 1,
            },
            MethodInfo {
                name: "EnableCommand", slot: 13, base: "ITimeCtl", ret: "void",
                params: &["ITimeCtl*", "boolean", "uint16"],
                words: 3,
            },
            MethodInfo {
                name: "GetTimeString", slot: 14, base: "ITimeCtl", ret: "void",
                params: &["ITimeCtl*", "uint32", "AECHAR*", "int", "uint16"],
                words: 5,
            },
            MethodInfo {
                name: "SetIncrement", slot: 15, base: "ITimeCtl", ret: "void",
                params: &["ITimeCtl*", "uint16"],
                words: 2,
            },
            MethodInfo {
                name: "SetEditField", slot: 16, base: "ITimeCtl", ret: "void",
                params: &["ITimeCtl*", "ITField"],
                words: 2,
            },
            MethodInfo {
                name: "SetFont", slot: 17, base: "ITimeCtl", ret: "void",
                params: &["ITimeCtl*", "AEEFont", "AEEFont"],
                words: 3,
            },
            MethodInfo {
                name: "GetFont", slot: 18, base: "ITimeCtl", ret: "boolean",
                params: &["ITimeCtl*", "AEEFont*", "AEEFont*"],
                words: 3,
            },
            MethodInfo {
                name: "SizeToFit", slot: 19, base: "ITimeCtl", ret: "void",
                params: &["ITimeCtl*", "AEERect*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "ITransform",
        base: "IQI",
        declared_in: "AEEITransform.h",
        iid: 0x01001029,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "TransformBltSimple", slot: 3, base: "ITransform", ret: "int",
                params: &["ITransform*", "int", "int", "IBitmap*", "int", "int", "unsigned", "unsigned", "uint16", "uint8"],
                words: 10,
            },
            MethodInfo {
                name: "TransformBltComplex", slot: 4, base: "ITransform", ret: "int",
                params: &["ITransform*", "int", "int", "IBitmap*", "int", "int", "unsigned", "unsigned", "AEETransformMatrix*", "uint8"],
                words: 10,
            },
        ],
    },
    IfaceInfo {
        name: "IUnzipAStream",
        base: "IBase",
        declared_in: "AEEUnzipStream.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Readable", slot: 2, base: "IAStream", ret: "void",
                params: &["IAStream*", "PFNNOTIFY", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "Read", slot: 3, base: "IAStream", ret: "int32",
                params: &["IAStream*", "void*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "Cancel", slot: 4, base: "IAStream", ret: "void",
                params: &["IAStream*", "PFNNOTIFY", "void*"],
                words: 3,
            },
            MethodInfo {
                name: "SetStream", slot: 5, base: "IUnzipAStream", ret: "void",
                params: &["IUnzipAStream*", "IAStream*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IVersion",
        base: "IQI",
        declared_in: "AEEIVersion.h",
        iid: 0x0104f5c0,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetVersion", slot: 3, base: "IVersion", ret: "int",
                params: &["IVersion*", "AEEVersionDesc*", "int"],
                words: 3,
            },
            MethodInfo {
                name: "GetInfoString", slot: 4, base: "IVersion", ret: "int",
                params: &["IVersion*", "int", "char*", "int", "int*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IVocoder",
        base: "IBase",
        declared_in: "AEEVocoder.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IBase", ret: "uint32",
                params: &["IBase*"],
                words: 1,
            },
            MethodInfo {
                name: "VocConfigure", slot: 2, base: "IVocoder", ret: "int16",
                params: &["IVocoder*", "IVocoderConfigType", "IVocoderInfoType*"],
                words: 3,
            },
            MethodInfo {
                name: "VocInStart", slot: 3, base: "IVocoder", ret: "int16",
                params: &["IVocoder*"],
                words: 1,
            },
            MethodInfo {
                name: "VocInStop", slot: 4, base: "IVocoder", ret: "int16",
                params: &["IVocoder*"],
                words: 1,
            },
            MethodInfo {
                name: "VocInReset", slot: 5, base: "IVocoder", ret: "int16",
                params: &["IVocoder*"],
                words: 1,
            },
            MethodInfo {
                name: "VocInRead", slot: 6, base: "IVocoder", ret: "int16",
                params: &["IVocoder*", "DataRateType*", "uint16*", "uint8*"],
                words: 4,
            },
            MethodInfo {
                name: "VocOutStart", slot: 7, base: "IVocoder", ret: "int16",
                params: &["IVocoder*"],
                words: 1,
            },
            MethodInfo {
                name: "VocOutStop", slot: 8, base: "IVocoder", ret: "int16",
                params: &["IVocoder*"],
                words: 1,
            },
            MethodInfo {
                name: "VocOutReset", slot: 9, base: "IVocoder", ret: "int16",
                params: &["IVocoder*"],
                words: 1,
            },
            MethodInfo {
                name: "VocOutWrite", slot: 10, base: "IVocoder", ret: "int16",
                params: &["IVocoder*", "DataRateType", "uint16", "uint8*"],
                words: 4,
            },
            MethodInfo {
                name: "GetOutDepth", slot: 11, base: "IVocoder", ret: "int16",
                params: &["IVocoder*", "uint16*"],
                words: 2,
            },
            MethodInfo {
                name: "GetInDepth", slot: 12, base: "IVocoder", ret: "int16",
                params: &["IVocoder*", "uint16*"],
                words: 2,
            },
            MethodInfo {
                name: "GetFrameLength", slot: 13, base: "IVocoder", ret: "int16",
                params: &["IVocoder*", "VocoderType", "DataRateType", "uint16*"],
                words: 4,
            },
            MethodInfo {
                name: "PlayTone", slot: 14, base: "IVocoder", ret: "int16",
                params: &["IVocoder*", "uint16", "uint16", "int16", "uint16"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IWIFI",
        base: "IQueryInterface",
        declared_in: "AEEWIFI.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "LoadOptions", slot: 3, base: "IWIFI", ret: "int",
                params: &["IWIFI*", "uint32", "IWIFIOpts*"],
                words: 3,
            },
            MethodInfo {
                name: "CommitOptions", slot: 4, base: "IWIFI", ret: "int",
                params: &["IWIFI*", "uint32", "IWIFIOpts*"],
                words: 3,
            },
            MethodInfo {
                name: "GetStatistics", slot: 5, base: "IWIFI", ret: "int",
                params: &["IWIFI*", "AEEWIFIStatistics*"],
                words: 2,
            },
            MethodInfo {
                name: "EnumProfileInit", slot: 6, base: "IWIFI", ret: "int",
                params: &["IWIFI*"],
                words: 1,
            },
            MethodInfo {
                name: "EnumNextProfile", slot: 7, base: "IWIFI", ret: "int",
                params: &["IWIFI*", "uint32*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IWIFIOpts",
        base: "IQueryInterface",
        declared_in: "AEEWIFI.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IWIFIOpts", ret: "int",
                params: &["IWIFIOpts*", "WIFIOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IWIFIOpts", ret: "int",
                params: &["IWIFIOpts*", "int32"],
                words: 2,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IWIFIOpts", ret: "int",
                params: &["IWIFIOpts*", "int32", "WIFIOpt*"],
                words: 3,
            },
        ],
    },
    IfaceInfo {
        name: "IWeb",
        base: "IWebOpts",
        declared_in: "AEEWeb.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "GetResponse", slot: 6, base: "IWeb", ret: "void",
                params: &["IWeb*", "IWebResp**", "AEECallback*", "char*", "..."],
                words: 5,
            },
            MethodInfo {
                name: "GetResponseV", slot: 7, base: "IWeb", ret: "void",
                params: &["IWeb*", "IWebResp**", "AEECallback*", "char*", "WebOpt*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IWebEng",
        base: "IQueryInterface",
        declared_in: "AEEWeb.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "Transaction", slot: 3, base: "IWebEng", ret: "int",
                params: &["IWebEng*", "IWeb*", "IWebReq*", "AEECallback*", "IWebResp**"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IWebReq",
        base: "IWebOpts",
        declared_in: "AEEWeb.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "GetUrl", slot: 6, base: "IWebReq", ret: "char*",
                params: &["IWebReq*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IWebResp",
        base: "IWebOpts",
        declared_in: "AEEWeb.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "GetInfo", slot: 6, base: "IWebResp", ret: "WebRespInfo*",
                params: &["IWebResp*"],
                words: 1,
            },
        ],
    },
    IfaceInfo {
        name: "IWebUtil",
        base: "IQueryInterface",
        declared_in: "AEEWeb.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "ParseUrl", slot: 3, base: "IWebUtil", ret: "int",
                params: &["IWebUtil*", "char*", "UrlParts*"],
                words: 3,
            },
            MethodInfo {
                name: "MakeUrl", slot: 4, base: "IWebUtil", ret: "int",
                params: &["IWebUtil*", "char*", "char*", "char*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "UrlEncode", slot: 5, base: "IWebUtil", ret: "char*",
                params: &["IWebUtil*", "char*", "int*", "char*", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "UrlDecode", slot: 6, base: "IWebUtil", ret: "char*",
                params: &["IWebUtil*", "char*", "int*", "char*", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "EncodeBase64", slot: 7, base: "IWebUtil", ret: "char*",
                params: &["IWebUtil*", "char*", "int", "char*"],
                words: 4,
            },
            MethodInfo {
                name: "DecodeBase64", slot: 8, base: "IWebUtil", ret: "int",
                params: &["IWebUtil*", "char*", "char*"],
                words: 3,
            },
            MethodInfo {
                name: "ParseFormFields", slot: 9, base: "IWebUtil", ret: "int",
                params: &["IWebUtil*", "char**", "WebFormField*", "int", "uint32"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IWindowMgr0",
        base: "IQI",
        declared_in: "AEEIWindowMgr0.h",
        iid: 0x0103da14,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetExtent", slot: 3, base: "IWindowMgr0", ret: "int",
                params: &["IWindowMgr0*", "AEEExtent*"],
                words: 2,
            },
            MethodInfo {
                name: "CreateWindow", slot: 4, base: "IWindowMgr0", ret: "int",
                params: &["IWindowMgr0*", "IQI*", "uint32"],
                words: 3,
            },
            MethodInfo {
                name: "SetWindowProp", slot: 5, base: "IWindowMgr0", ret: "int",
                params: &["IWindowMgr0*", "IQI*", "AEECLSID", "byte*", "int"],
                words: 5,
            },
            MethodInfo {
                name: "ActivateWindow", slot: 6, base: "IWindowMgr0", ret: "int",
                params: &["IWindowMgr0*", "IQI*"],
                words: 2,
            },
            MethodInfo {
                name: "CloseWindow", slot: 7, base: "IWindowMgr0", ret: "int",
                params: &["IWindowMgr0*", "IQI*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IX509Chain",
        base: "IWebOpts",
        declared_in: "AEEX509Chain.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
            MethodInfo {
                name: "AddCert", slot: 6, base: "IX509Chain", ret: "int",
                params: &["IX509Chain*", "int32", "uint8*", "int"],
                words: 4,
            },
            MethodInfo {
                name: "GetBasic", slot: 7, base: "IX509Chain", ret: "int",
                params: &["IX509Chain*", "int32", "int32", "X509BasicCert*"],
                words: 4,
            },
            MethodInfo {
                name: "GetField", slot: 8, base: "IX509Chain", ret: "int",
                params: &["IX509Chain*", "int32", "int32", "int", "uint8**", "unsigned int*"],
                words: 6,
            },
            MethodInfo {
                name: "GetFieldPart", slot: 9, base: "IX509Chain", ret: "int",
                params: &["IX509Chain*", "X509PartRequest*", "X509CertPart*"],
                words: 3,
            },
            MethodInfo {
                name: "GetRSAKey", slot: 10, base: "IX509Chain", ret: "int",
                params: &["IX509Chain*", "int32", "int32", "IRSA**"],
                words: 4,
            },
            MethodInfo {
                name: "Verify", slot: 11, base: "IX509Chain", ret: "void",
                params: &["IX509Chain*", "X509TrustResult*", "AEECallback*", "..."],
                words: 4,
            },
            MethodInfo {
                name: "VerifyV", slot: 12, base: "IX509Chain", ret: "void",
                params: &["IX509Chain*", "X509TrustResult*", "AEECallback*", "WebOpt*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IvCalObject",
        base: "IvObject",
        declared_in: "AEEIvCalObject.h",
        iid: 0x0105ccc5,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetObjectType", slot: 3, base: "IvObject", ret: "int",
                params: &["IvObject*", "AEECLSID*"],
                words: 2,
            },
            MethodInfo {
                name: "AddProperty", slot: 4, base: "IvCalObject", ret: "int",
                params: &["IvCalObject*", "IvProperty*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveProperty", slot: 5, base: "IvCalObject", ret: "int",
                params: &["IvCalObject*", "IvProperty*"],
                words: 2,
            },
            MethodInfo {
                name: "GetNumberOfProperties", slot: 6, base: "IvCalObject", ret: "int",
                params: &["IvCalObject*", "AEEVProperty", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "GetPropertyAtIndex", slot: 7, base: "IvCalObject", ret: "int",
                params: &["IvCalObject*", "AEEVProperty", "int", "IvProperty**"],
                words: 4,
            },
            MethodInfo {
                name: "GetNumberOfObjects", slot: 8, base: "IvCalObject", ret: "int",
                params: &["IvCalObject*", "AEECLSID", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "GetObjectAtIndex", slot: 9, base: "IvCalObject", ret: "int",
                params: &["IvCalObject*", "int", "AEECLSID", "IvCalObject**"],
                words: 4,
            },
            MethodInfo {
                name: "AddObject", slot: 10, base: "IvCalObject", ret: "int",
                params: &["IvCalObject*", "IvCalObject*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveObject", slot: 11, base: "IvCalObject", ret: "int",
                params: &["IvCalObject*", "IvCalObject*"],
                words: 2,
            },
            MethodInfo {
                name: "SetVersion", slot: 12, base: "IvCalObject", ret: "int",
                params: &["IvCalObject*", "AEEVCalVersion"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IvCalStore",
        base: "IQI",
        declared_in: "AEEIvCalStore.h",
        iid: 0x0105f4ad,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddObject", slot: 3, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "IvCalObject*", "AEEVProperty*"],
                words: 3,
            },
            MethodInfo {
                name: "UpdateObject", slot: 4, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "IvCalObject*"],
                words: 2,
            },
            MethodInfo {
                name: "GetObjectByOID", slot: 5, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "AEEVProperty", "IvCalObject**"],
                words: 3,
            },
            MethodInfo {
                name: "DeleteObjectByOID", slot: 6, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "AEEVProperty"],
                words: 2,
            },
            MethodInfo {
                name: "GetObjectOIDs", slot: 7, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "AEECLSID", "AEEVProperty*", "int", "int*"],
                words: 5,
            },
            MethodInfo {
                name: "GetObjectOIDByUID", slot: 8, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "char*", "AEEVProperty*"],
                words: 3,
            },
            MethodInfo {
                name: "LockCalendar", slot: 9, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*"],
                words: 1,
            },
            MethodInfo {
                name: "UnlockCalendar", slot: 10, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*"],
                words: 1,
            },
            MethodInfo {
                name: "IsCalendarLocked", slot: 11, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "boolean*"],
                words: 2,
            },
            MethodInfo {
                name: "OnAdd", slot: 12, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "OnUpdate", slot: 13, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "OnDelete", slot: 14, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "ISignal*"],
                words: 2,
            },
            MethodInfo {
                name: "OnLock", slot: 15, base: "IvCalStore", ret: "int",
                params: &["IvCalStore*", "ISignal*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IvCard",
        base: "IvObject",
        declared_in: "AEEIvCard.h",
        iid: 0x0105ccd6,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetObjectType", slot: 3, base: "IvObject", ret: "int",
                params: &["IvObject*", "AEECLSID*"],
                words: 2,
            },
            MethodInfo {
                name: "AddProperty", slot: 4, base: "IvCard", ret: "int",
                params: &["IvCard*", "IvProperty*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveProperty", slot: 5, base: "IvCard", ret: "int",
                params: &["IvCard*", "IvProperty*"],
                words: 2,
            },
            MethodInfo {
                name: "GetNumberOfProperties", slot: 6, base: "IvCard", ret: "int",
                params: &["IvCard*", "AEEVProperty", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "GetPropertyAtIndex", slot: 7, base: "IvCard", ret: "int",
                params: &["IvCard*", "AEEVProperty", "int", "IvProperty**"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IvObject",
        base: "IQI",
        declared_in: "AEEIvObject.h",
        iid: 0x010627be,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetObjectType", slot: 3, base: "IvObject", ret: "int",
                params: &["IvObject*", "AEECLSID*"],
                words: 2,
            },
        ],
    },
    IfaceInfo {
        name: "IvObjectParser",
        base: "IQI",
        declared_in: "AEEIvObjectParser.h",
        iid: 0x0105f45d,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetNumberOfParsedObjects", slot: 3, base: "IvObjectParser", ret: "int",
                params: &["IvObjectParser*", "int*", "AEECLSID*"],
                words: 3,
            },
            MethodInfo {
                name: "GetObjectAtIndex", slot: 4, base: "IvObjectParser", ret: "int",
                params: &["IvObjectParser*", "int", "IvObject**"],
                words: 3,
            },
            MethodInfo {
                name: "GetBufferFromIvObject", slot: 5, base: "IvObjectParser", ret: "int",
                params: &["IvObjectParser*", "IvObject*", "char*", "int", "int*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IvParm",
        base: "IvObject",
        declared_in: "AEEIvParm.h",
        iid: 0x0105d01e,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetObjectType", slot: 3, base: "IvObject", ret: "int",
                params: &["IvObject*", "AEECLSID*"],
                words: 2,
            },
            MethodInfo {
                name: "GetNumberOfValues", slot: 4, base: "IvParm", ret: "int",
                params: &["IvParm*", "int*"],
                words: 2,
            },
            MethodInfo {
                name: "GetId", slot: 5, base: "IvParm", ret: "int",
                params: &["IvParm*", "AEEVParameter*"],
                words: 2,
            },
            MethodInfo {
                name: "GetParmName", slot: 6, base: "IvParm", ret: "int",
                params: &["IvParm*", "char*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "InitParm", slot: 7, base: "IvParm", ret: "int",
                params: &["IvParm*", "AEEVParameter", "char*", "uint32", "AECHAR*"],
                words: 5,
            },
            MethodInfo {
                name: "AddValue", slot: 8, base: "IvParm", ret: "int",
                params: &["IvParm*", "uint32", "AECHAR*"],
                words: 3,
            },
            MethodInfo {
                name: "ParseValue", slot: 9, base: "IvParm", ret: "int",
                params: &["IvParm*", "boolean", "int", "uint32*", "void*", "int", "int*"],
                words: 7,
            },
        ],
    },
    IfaceInfo {
        name: "IvProperty",
        base: "IvObject",
        declared_in: "AEEIvProperty.h",
        iid: 0x0105d01d,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "GetObjectType", slot: 3, base: "IvObject", ret: "int",
                params: &["IvObject*", "AEECLSID*"],
                words: 2,
            },
            MethodInfo {
                name: "AddParm", slot: 4, base: "IvProperty", ret: "int",
                params: &["IvProperty*", "IvParm*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveParm", slot: 5, base: "IvProperty", ret: "int",
                params: &["IvProperty*", "IvParm*"],
                words: 2,
            },
            MethodInfo {
                name: "GetNumberOfParms", slot: 6, base: "IvProperty", ret: "int",
                params: &["IvProperty*", "AEEVParameter", "int*"],
                words: 3,
            },
            MethodInfo {
                name: "GetParmAtIndex", slot: 7, base: "IvProperty", ret: "int",
                params: &["IvProperty*", "AEEVParameter", "int", "IvParm**"],
                words: 4,
            },
            MethodInfo {
                name: "GetId", slot: 8, base: "IvProperty", ret: "int",
                params: &["IvProperty*", "AEEVProperty*"],
                words: 2,
            },
            MethodInfo {
                name: "InitProperty", slot: 9, base: "IvProperty", ret: "int",
                params: &["IvProperty*", "AEEVProperty", "char*", "boolean", "void*", "int", "char*"],
                words: 7,
            },
            MethodInfo {
                name: "GetPropertyName", slot: 10, base: "IvProperty", ret: "int",
                params: &["IvProperty*", "char*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "GetGroupName", slot: 11, base: "IvProperty", ret: "int",
                params: &["IvProperty*", "char*", "int", "int*"],
                words: 4,
            },
            MethodInfo {
                name: "ParseValue", slot: 12, base: "IvProperty", ret: "int",
                params: &["IvProperty*", "boolean", "void*", "int", "int*"],
                words: 5,
            },
        ],
    },
    IfaceInfo {
        name: "IxOpts",
        base: "IQI",
        declared_in: "AEEIxOpts.h",
        iid: 0x0102c269,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "AddOpt", slot: 3, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "xOpt*"],
                words: 2,
            },
            MethodInfo {
                name: "RemoveOpt", slot: 4, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32"],
                words: 3,
            },
            MethodInfo {
                name: "GetOpt", slot: 5, base: "IxOpts", ret: "int",
                params: &["IxOpts*", "int32", "int32", "xOpt*"],
                words: 4,
            },
        ],
    },
    IfaceInfo {
        name: "IGL",
        base: "IQueryInterface",
        declared_in: "AEEGL.h",
        iid: NONE_IID,
        methods: &[
            MethodInfo {
                name: "AddRef", slot: 0, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "Release", slot: 1, base: "IQI", ret: "uint32",
                params: &["IQI*"],
                words: 1,
            },
            MethodInfo {
                name: "QueryInterface", slot: 2, base: "IQI", ret: "int",
                params: &["IQI*", "AEEIID", "void**"],
                words: 3,
            },
            MethodInfo {
                name: "ActiveTexture", slot: 3, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "AlphaFuncx", slot: 4, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLclampx"],
                words: 2,
            },
            MethodInfo {
                name: "BindTexture", slot: 5, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLuint"],
                words: 2,
            },
            MethodInfo {
                name: "BlendFunc", slot: 6, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "Clear", slot: 7, base: "IGL", ret: "int",
                params: &["AEEGLbitfield"],
                words: 1,
            },
            MethodInfo {
                name: "ClearColorx", slot: 8, base: "IGL", ret: "int",
                params: &["AEEGLclampx", "AEEGLclampx", "AEEGLclampx", "AEEGLclampx"],
                words: 4,
            },
            MethodInfo {
                name: "ClearDepthx", slot: 9, base: "IGL", ret: "int",
                params: &["AEEGLclampx"],
                words: 1,
            },
            MethodInfo {
                name: "ClearStencil", slot: 10, base: "IGL", ret: "int",
                params: &["AEEGLint"],
                words: 1,
            },
            MethodInfo {
                name: "ClientActiveTexture", slot: 11, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "Color4x", slot: 12, base: "IGL", ret: "int",
                params: &["AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "ColorMask", slot: 13, base: "IGL", ret: "int",
                params: &["AEEGLboolean", "AEEGLboolean", "AEEGLboolean", "AEEGLboolean"],
                words: 4,
            },
            MethodInfo {
                name: "ColorPointer", slot: 14, base: "IGL", ret: "int",
                params: &["AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 4,
            },
            MethodInfo {
                name: "CompressedTexImage2D", slot: 15, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLsizei", "AEEGLint", "AEEGLsizei", "AEEGLvoid*"],
                words: 8,
            },
            MethodInfo {
                name: "CompressedTexSubImage2D", slot: 16, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 9,
            },
            MethodInfo {
                name: "CopyTexImage2D", slot: 17, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint", "AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLint"],
                words: 8,
            },
            MethodInfo {
                name: "CopyTexSubImage2D", slot: 18, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei"],
                words: 8,
            },
            MethodInfo {
                name: "CullFace", slot: 19, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "DeleteTextures", slot: 20, base: "IGL", ret: "int",
                params: &["AEEGLsizei", "AEEGLuint*"],
                words: 2,
            },
            MethodInfo {
                name: "DepthFunc", slot: 21, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "DepthMask", slot: 22, base: "IGL", ret: "int",
                params: &["AEEGLboolean"],
                words: 1,
            },
            MethodInfo {
                name: "DepthRangex", slot: 23, base: "IGL", ret: "int",
                params: &["AEEGLclampx", "AEEGLclampx"],
                words: 2,
            },
            MethodInfo {
                name: "Disable", slot: 24, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "DisableClientState", slot: 25, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "DrawArrays", slot: 26, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint", "AEEGLsizei"],
                words: 3,
            },
            MethodInfo {
                name: "DrawElements", slot: 27, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLsizei", "AEEGLenum", "AEEGLvoid*"],
                words: 4,
            },
            MethodInfo {
                name: "Enable", slot: 28, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "EnableClientState", slot: 29, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "Finish", slot: 30, base: "IGL", ret: "int",
                params: &[],
                words: 0,
            },
            MethodInfo {
                name: "Flush", slot: 31, base: "IGL", ret: "int",
                params: &[],
                words: 0,
            },
            MethodInfo {
                name: "Fogx", slot: 32, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfixed"],
                words: 2,
            },
            MethodInfo {
                name: "Fogxv", slot: 33, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfixed*"],
                words: 2,
            },
            MethodInfo {
                name: "FrontFace", slot: 34, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "Frustumx", slot: 35, base: "IGL", ret: "int",
                params: &["AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 6,
            },
            MethodInfo {
                name: "GenTextures", slot: 36, base: "IGL", ret: "int",
                params: &["AEEGLsizei", "AEEGLuint*"],
                words: 2,
            },
            MethodInfo {
                name: "GetError", slot: 37, base: "IGL", ret: "int",
                params: &["AEEGLenum*"],
                words: 1,
            },
            MethodInfo {
                name: "GetIntegerv", slot: 38, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint*"],
                words: 2,
            },
            MethodInfo {
                name: "GetString", slot: 39, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLubyte**"],
                words: 2,
            },
            MethodInfo {
                name: "Hint", slot: 40, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum"],
                words: 2,
            },
            MethodInfo {
                name: "LightModelx", slot: 41, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfixed"],
                words: 2,
            },
            MethodInfo {
                name: "LightModelxv", slot: 42, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfixed*"],
                words: 2,
            },
            MethodInfo {
                name: "Lightx", slot: 43, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "Lightxv", slot: 44, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "LineWidthx", slot: 45, base: "IGL", ret: "int",
                params: &["AEEGLfixed"],
                words: 1,
            },
            MethodInfo {
                name: "LoadIdentity", slot: 46, base: "IGL", ret: "int",
                params: &[],
                words: 0,
            },
            MethodInfo {
                name: "LoadMatrixx", slot: 47, base: "IGL", ret: "int",
                params: &["AEEGLfixed*"],
                words: 1,
            },
            MethodInfo {
                name: "LogicOp", slot: 48, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "Materialx", slot: 49, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "Materialxv", slot: 50, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "MatrixMode", slot: 51, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "MultMatrixx", slot: 52, base: "IGL", ret: "int",
                params: &["AEEGLfixed*"],
                words: 1,
            },
            MethodInfo {
                name: "MultiTexCoord4x", slot: 53, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 5,
            },
            MethodInfo {
                name: "Normal3x", slot: 54, base: "IGL", ret: "int",
                params: &["AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "NormalPointer", slot: 55, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 3,
            },
            MethodInfo {
                name: "Orthox", slot: 56, base: "IGL", ret: "int",
                params: &["AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 6,
            },
            MethodInfo {
                name: "PixelStorei", slot: 57, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint"],
                words: 2,
            },
            MethodInfo {
                name: "PointSizex", slot: 58, base: "IGL", ret: "int",
                params: &["AEEGLfixed"],
                words: 1,
            },
            MethodInfo {
                name: "PolygonOffsetx", slot: 59, base: "IGL", ret: "int",
                params: &["AEEGLfixed", "AEEGLfixed"],
                words: 2,
            },
            MethodInfo {
                name: "PopMatrix", slot: 60, base: "IGL", ret: "int",
                params: &[],
                words: 0,
            },
            MethodInfo {
                name: "PushMatrix", slot: 61, base: "IGL", ret: "int",
                params: &[],
                words: 0,
            },
            MethodInfo {
                name: "ReadPixels", slot: 62, base: "IGL", ret: "int",
                params: &["AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLenum", "AEEGLenum", "AEEGLvoid*"],
                words: 7,
            },
            MethodInfo {
                name: "Rotatex", slot: 63, base: "IGL", ret: "int",
                params: &["AEEGLfixed", "AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 4,
            },
            MethodInfo {
                name: "SampleCoveragex", slot: 64, base: "IGL", ret: "int",
                params: &["AEEGLclampx", "AEEGLboolean"],
                words: 2,
            },
            MethodInfo {
                name: "Scalex", slot: 65, base: "IGL", ret: "int",
                params: &["AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "Scissor", slot: 66, base: "IGL", ret: "int",
                params: &["AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei"],
                words: 4,
            },
            MethodInfo {
                name: "ShadeModel", slot: 67, base: "IGL", ret: "int",
                params: &["AEEGLenum"],
                words: 1,
            },
            MethodInfo {
                name: "StencilFunc", slot: 68, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint", "AEEGLuint"],
                words: 3,
            },
            MethodInfo {
                name: "StencilMask", slot: 69, base: "IGL", ret: "int",
                params: &["AEEGLuint"],
                words: 1,
            },
            MethodInfo {
                name: "StencilOp", slot: 70, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLenum"],
                words: 3,
            },
            MethodInfo {
                name: "TexCoordPointer", slot: 71, base: "IGL", ret: "int",
                params: &["AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 4,
            },
            MethodInfo {
                name: "TexEnvx", slot: 72, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "TexEnvxv", slot: 73, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
            MethodInfo {
                name: "TexImage2D", slot: 74, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLint", "AEEGLenum", "AEEGLenum", "AEEGLvoid*"],
                words: 9,
            },
            MethodInfo {
                name: "TexParameterx", slot: 75, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "TexSubImage2D", slot: 76, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLint", "AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei", "AEEGLenum", "AEEGLenum", "AEEGLvoid*"],
                words: 9,
            },
            MethodInfo {
                name: "Translatex", slot: 77, base: "IGL", ret: "int",
                params: &["AEEGLfixed", "AEEGLfixed", "AEEGLfixed"],
                words: 3,
            },
            MethodInfo {
                name: "VertexPointer", slot: 78, base: "IGL", ret: "int",
                params: &["AEEGLint", "AEEGLenum", "AEEGLsizei", "AEEGLvoid*"],
                words: 4,
            },
            MethodInfo {
                name: "Viewport", slot: 79, base: "IGL", ret: "int",
                params: &["AEEGLint", "AEEGLint", "AEEGLsizei", "AEEGLsizei"],
                words: 4,
            },
            MethodInfo {
                name: "AlphaFunc", slot: 80, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLclampf"],
                words: 2,
            },
            MethodInfo {
                name: "ClearColor", slot: 81, base: "IGL", ret: "int",
                params: &["AEEGLclampf", "AEEGLclampf", "AEEGLclampf", "AEEGLclampf"],
                words: 4,
            },
            MethodInfo {
                name: "ClearDepthf", slot: 82, base: "IGL", ret: "int",
                params: &["AEEGLclampf"],
                words: 1,
            },
            MethodInfo {
                name: "Color4f", slot: 83, base: "IGL", ret: "int",
                params: &["AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "DepthRangef", slot: 84, base: "IGL", ret: "int",
                params: &["AEEGLclampf", "AEEGLclampf"],
                words: 2,
            },
            MethodInfo {
                name: "Fogf", slot: 85, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfloat"],
                words: 2,
            },
            MethodInfo {
                name: "Fogfv", slot: 86, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfloat*"],
                words: 2,
            },
            MethodInfo {
                name: "Frustumf", slot: 87, base: "IGL", ret: "int",
                params: &["AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 6,
            },
            MethodInfo {
                name: "Lightf", slot: 88, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "Lightfv", slot: 89, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "LightModelf", slot: 90, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfloat"],
                words: 2,
            },
            MethodInfo {
                name: "LightModelfv", slot: 91, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfloat*"],
                words: 2,
            },
            MethodInfo {
                name: "LineWidth", slot: 92, base: "IGL", ret: "int",
                params: &["AEEGLfloat"],
                words: 1,
            },
            MethodInfo {
                name: "LoadMatrixf", slot: 93, base: "IGL", ret: "int",
                params: &["AEEGLfloat*"],
                words: 1,
            },
            MethodInfo {
                name: "Materialf", slot: 94, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "Materialfv", slot: 95, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "MultiTexCoord4f", slot: 96, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 5,
            },
            MethodInfo {
                name: "MultMatrixf", slot: 97, base: "IGL", ret: "int",
                params: &["AEEGLfloat*"],
                words: 1,
            },
            MethodInfo {
                name: "Normal3f", slot: 98, base: "IGL", ret: "int",
                params: &["AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "Orthof", slot: 99, base: "IGL", ret: "int",
                params: &["AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 6,
            },
            MethodInfo {
                name: "PointSize", slot: 100, base: "IGL", ret: "int",
                params: &["AEEGLfloat"],
                words: 1,
            },
            MethodInfo {
                name: "PolygonOffset", slot: 101, base: "IGL", ret: "int",
                params: &["AEEGLfloat", "AEEGLfloat"],
                words: 2,
            },
            MethodInfo {
                name: "Rotatef", slot: 102, base: "IGL", ret: "int",
                params: &["AEEGLfloat", "AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 4,
            },
            MethodInfo {
                name: "SampleCoverage", slot: 103, base: "IGL", ret: "int",
                params: &["AEEGLclampf", "AEEGLboolean"],
                words: 2,
            },
            MethodInfo {
                name: "Scalef", slot: 104, base: "IGL", ret: "int",
                params: &["AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "TexEnvf", slot: 105, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "TexEnvfv", slot: 106, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "TexParameterf", slot: 107, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "Translatef", slot: 108, base: "IGL", ret: "int",
                params: &["AEEGLfloat", "AEEGLfloat", "AEEGLfloat"],
                words: 3,
            },
            MethodInfo {
                name: "TexParameterfv", slot: 109, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfloat*"],
                words: 3,
            },
            MethodInfo {
                name: "TexParameterxv", slot: 110, base: "IGL", ret: "int",
                params: &["AEEGLenum", "AEEGLenum", "AEEGLfixed*"],
                words: 3,
            },
        ],
    },
];

pub const IFACE_COUNT: usize = 197;
pub const VTABLE_SLOT_TOTAL: usize = 2484;
pub static IFACE_ALIAS: &[(&str, &str)] = &[
    ("ICallOrigOpts", "IWebOpts"),
    ("ICoreNotifier", "INotifier"),
    ("IGetLine", "IGetline"),
    ("IHashCTX", "IHashCtx"),
    ("IMediaAAC", "IMedia"),
    ("IMediaADPCM", "IMedia"),
    ("IMediaMIDI", "IMedia"),
    ("IMediaMIDIOutMsg", "IMedia"),
    ("IMediaMIDIOutQCP", "IMedia"),
    ("IMediaMMF", "IMedia"),
    ("IMediaMP3", "IMedia"),
    ("IMediaMPEG4", "IMedia"),
    ("IMediaPMD", "IMedia"),
    ("IMediaPhr", "IMedia"),
    ("IMediaQCP", "IMedia"),
    ("IQueryInterface", "IQI"),
    ("ISMSBCSrvOpts", "IWebOpts"),
    ("ISMSMsg", "IWebOpts"),
    ("IWebOpts", "IxOpts"),
];

pub static IID_TO_IFACE: &[(u32, &str)] = &[
    (0x01000001, "QUERYINTERFACE"),
    (0x01001012, "ISource"),
    (0x01001018, "IPort"),
    (0x0100101c, "IPeek"),
    (0x01001021, "IBitmap"),
    (0x01001022, "IFont"),
    (0x01001029, "ITransform"),
    (0x0100102c, "DIB_20"),
    (0x01001045, "IDIB"),
    (0x01001056, "ISound"),
    (0x01005006, "IDNS"),
    (0x01005500, "IMedia"),
    (0x01010100, "IApplet"),
    (0x01012719, "IBitmapDev"),
    (0x01013110, "IMAGE"),
    (0x0101593a, "MODEL"),
    (0x0101a114, "IFilePort"),
    (0x0101adf9, "IFileSystem"),
    (0x0101c8af, "IDNSConfig"),
    (0x0101e52e, "RESOURCECTL"),
    (0x0101e8bc, "IBitmapScale"),
    (0x0101eb0b, "IForceFeed"),
    (0x0101ed1f, "ISingleton"),
    (0x0102345e, "IRealloc"),
    (0x0102346d, "IEnv"),
    (0x01024f18, "IDispatcher"),
    (0x01026e20, "IMAGEDECODER"),
    (0x010285f5, "ISignal"),
    (0x0102b6c6, "IQoSSpec"),
    (0x0102b6ca, "IQoSList"),
    (0x0102b6cb, "IQoSFlow"),
    (0x0102b6cc, "IQoSFilter"),
    (0x0102bba1, "IMod"),
    (0x0102c269, "IxOpts"),
    (0x0102cb09, "IHashCtx"),
    (0x0102ccdf, "ICipherWrapper"),
    (0x0102cce3, "ICipher1"),
    (0x0102d13b, "IParameters"),
    (0x0102d13c, "IParametersRO"),
    (0x0102d154, "RAWBLOCKCIPHER"),
    (0x0102eae4, "ICertChain"),
    (0x0102eaed, "IBN"),
    (0x0102eaee, "RSA"),
    (0x0102f959, "IPrivSet"),
    (0x0102fa3e, "LOCALSTORAGE"),
    (0x0102fd28, "IDNSConfig2"),
    (0x0102fefd, "ALARMMGR"),
    (0x010301ad, "MEDIADLS"),
    (0x010301af, "DLSLINKER"),
    (0x01030214, "IFLIP"),
    (0x01031dfc, "MCASTSESSION"),
    (0x010329cf, "INetUtils"),
    (0x01034502, "IPubKey"),
    (0x01034503, "IPubKeyEng"),
    (0x0103450b, "IPubKeyUtil"),
    (0x01035f45, "NETWORK"),
    (0x0103615e, "IRecordStore"),
    (0x0103615f, "IRamCache"),
    (0x01036173, "IBCMCSDB"),
    (0x010367a7, "IRscPool"),
    (0x010367a8, "IClipboard"),
    (0x01036a7b, "IBase"),
    (0x01036d36, "IDisplay"),
    (0x01036e17, "IAddrInfo"),
    (0x01037ad4, "QOSBUNDLE"),
    (0x01037ae6, "IAddrInfoCache"),
    (0x010386d5, "ICertBag"),
    (0x01038cc8, "SMSSTORAGE2"),
    (0x0103937c, "ICertParse"),
    (0x010396bf, "IAppHistory"),
    (0x01039ba8, "ISockPort"),
    (0x01039dfa, "BT"),
    (0x0103a3f1, "IBNEng"),
    (0x0103a4d8, "IAppletCtl"),
    (0x0103bce1, "IQoSSession"),
    (0x0103d14b, "ISignalBus"),
    (0x0103d8dd, "GLES10"),
    (0x0103d8de, "GLES10EXT"),
    (0x0103d8ea, "GLES11"),
    (0x0103d8eb, "GLES11EXT"),
    (0x0103d8ed, "EGL10"),
    (0x0103d8ee, "EGL11"),
    (0x0103d8ef, "EGLGETCOLORBUFFER"),
    (0x0103d8f0, "EGLGETPOWERLEVEL"),
    (0x0103da14, "IWindowMgr0"),
    (0x0103def1, "GLES11EXTPAK"),
    (0x0103e8d9, "BITMAPROT"),
    (0x0103f099, "IBitmapDev2"),
    (0x0104047f, "IParameters1"),
    (0x01041079, "ISignalCtl"),
    (0x01041694, "BT_SPP_SERVER"),
    (0x01041695, "BT_SPP_CLIENT"),
    (0x01041696, "BT_DEV_DB"),
    (0x0104171d, "ICipherFactory"),
    (0x010426e3, "EGLOESSWAPINTERVAL"),
    (0x0104340b, "BTSPP"),
    (0x010434cc, "EGLSURFACEMANIP_V1"),
    (0x01043541, "ISignalCBFactory"),
    (0x01044104, "ICharsetConv"),
    (0x010459b1, "GLESIMAGEONEXT_V1"),
    (0x01046312, "IBearerTechnology"),
    (0x0104f5c0, "IVersion"),
    (0x01051834, "EGLSURFACEMANIP"),
    (0x0105397c, "IBTMgr"),
    (0x01053c88, "IOBEXSrv"),
    (0x01053c89, "IOBEXCli"),
    (0x010570e0, "IBTServiceDiscovery"),
    (0x01058546, "GLESIMAGEONEXT"),
    (0x01058559, "IOBEXSrvSession"),
    (0x01058640, "IOBEXHeader"),
    (0x0105a9ad, "IBTConnection"),
    (0x0105aa38, "IBTAudioGateway"),
    (0x0105ccc5, "IvCalObject"),
    (0x0105ccd6, "IvCard"),
    (0x0105d01d, "IvProperty"),
    (0x0105d01e, "IvParm"),
    (0x0105f45d, "IvObjectParser"),
    (0x0105f4ad, "IvCalStore"),
    (0x01060f39, "IItemStore"),
    (0x01061c2a, "IBTA2DPNotifier"),
    (0x01061c2b, "IBTA2DP"),
    (0x01062245, "IRemoteControl_Target"),
    (0x01062249, "IBTAVRCPTarget"),
    (0x010627be, "IvObject"),
    (0x0106c38d, "IHID"),
    (0x0106c38e, "IHIDDevice"),
    (0x0109b10e, "EGLGETCOLORBUFFERINFO"),
];

