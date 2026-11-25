// @generated from local ABI definitions. DO NOT EDIT.
// Re-run the generator instead of patching this file.
#![allow(dead_code, non_upper_case_globals, non_snake_case)]
use crate::abi::{FieldInfo, StructInfo};

pub static STRUCTS: &[StructInfo] = &[
    StructInfo {
        name: "AEEAddrEnumRecInfo", size: 40, align: 4, packed: false, declared_in: "AEEAddrBook.h",
        fields: &[FieldInfo { name: "pCatList", off: 0, size: 4, ty: "AEEAddrCat*", bits: 0, bit_off: 0 }, FieldInfo { name: "nCategories", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pFieldList", off: 8, size: 4, ty: "AEEAddrFieldID*", bits: 0, bit_off: 0 }, FieldInfo { name: "nFields", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pTypeList", off: 16, size: 4, ty: "AEEAddrFieldType*", bits: 0, bit_off: 0 }, FieldInfo { name: "nTypes", off: 20, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 24, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwDataSize", off: 28, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "fType", off: 32, size: 4, ty: "AEEAddrFieldType", bits: 0, bit_off: 0 }, FieldInfo { name: "flags", off: 36, size: 4, ty: "AEEAddrSearchFlags", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEAddrField", size: 16, align: 4, packed: false, declared_in: "AEEAddrBook.h",
        fields: &[FieldInfo { name: "fID", off: 0, size: 2, ty: "AEEAddrFieldID", bits: 0, bit_off: 0 }, FieldInfo { name: "fType", off: 4, size: 4, ty: "AEEAddrFieldType", bits: 0, bit_off: 0 }, FieldInfo { name: "pBuffer", off: 8, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "wDataLen", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEAddrFieldInfo", size: 16, align: 4, packed: false, declared_in: "AEEAddrBook.h",
        fields: &[FieldInfo { name: "fieldID", off: 0, size: 2, ty: "AEEAddrFieldID", bits: 0, bit_off: 0 }, FieldInfo { name: "nMaxItems", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "fType", off: 8, size: 4, ty: "AEEAddrFieldType", bits: 0, bit_off: 0 }, FieldInfo { name: "bFieldSearch", off: 12, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bFieldCatSearch", off: 13, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEAddrInfo", size: 28, align: 4, packed: false, declared_in: "AEEIAddrInfo.h",
        fields: &[FieldInfo { name: "dwFlags", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nFamily", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nSockType", off: 8, size: 4, ty: "AEESockType", bits: 0, bit_off: 0 }, FieldInfo { name: "nProtocol", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "dwAddrLen", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "szCanonName", off: 20, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "psasAddr", off: 24, size: 4, ty: "union AEESockAddrStorage*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEAppInfo", size: 20, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "cls", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pszMIF", off: 4, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "wIDBase", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wAppType", off: 10, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wHostLo", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wHostHi", off: 14, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wIDPrivSet", off: 16, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wFlags", off: 18, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEAppStart", size: 24, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "error", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "clsApp", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pDisplay", off: 8, size: 4, ty: "IDisplay*", bits: 0, bit_off: 0 }, FieldInfo { name: "rc", off: 12, size: 8, ty: "AEERect", bits: 0, bit_off: 0 }, FieldInfo { name: "pszArgs", off: 20, size: 4, ty: "char*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEAppUsage", size: 12, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "dwRunCount", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwMinsOfUse", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwLastUsedTime", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEArc", size: 10, align: 2, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "cx", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "r", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "startAngle", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "arcAngle", off: 8, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEAutoRepeat", size: 12, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "wItem", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwStart", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwRate", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEBCMCSDBRecord_CompositeFlowId", size: 16, align: 4, packed: false, declared_in: "AEEIBCMCSDB.h",
        fields: &[FieldInfo { name: "sSimpleFlowId", off: 0, size: 4, ty: "AEEBCMCSDBRecord_SimpleFlowId", bits: 0, bit_off: 0 }, FieldInfo { name: "sProgramId", off: 4, size: 8, ty: "AEEBCMCSDBRecord_ProgramId", bits: 0, bit_off: 0 }, FieldInfo { name: "sFlowDiscrim", off: 12, size: 2, ty: "AEEBCMCSDBRecord_FlowDiscrim", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEBCMCSDBRecord_FlowDiscrim", size: 2, align: 1, packed: false, declared_in: "AEEIBCMCSDB.h",
        fields: &[FieldInfo { name: "val", off: 0, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "len", off: 1, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEBCMCSDBRecord_ProgramId", size: 8, align: 4, packed: false, declared_in: "AEEIBCMCSDB.h",
        fields: &[FieldInfo { name: "val", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "len", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEBTAVRCPEventType", size: 16, align: 4, packed: false, declared_in: "AEEIBTAVRCPTarget.h",
        fields: &[FieldInfo { name: "Event", off: 0, size: 4, ty: "AEEBTAVRCPEventEnum", bits: 0, bit_off: 0 }, FieldInfo { name: "Address", off: 4, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "Result", off: 12, size: 4, ty: "BTResult", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEBacklightInfo", size: 20, align: 4, packed: false, declared_in: "AEEBacklight.h",
        fields: &[FieldInfo { name: "dwSize", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwMinBrightnessLevel", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwMaxBrightnessLevel", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwDefaultBrightnessLevel", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBacklightColor", off: 16, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEBearerTechType", size: 8, align: 4, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "uBearerTech", off: 0, size: 4, ty: "AEEBearerTech", bits: 0, bit_off: 0 }, FieldInfo { name: "uBearerTechInfo", off: 4, size: 4, ty: "AEEBearerTechType@1", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEBearerTechType@1", size: 4, align: 4, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "acbCDMABearerTechInfo", off: 0, size: 4, ty: "AEECDMABearerTechInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "aubUMTSBearerTechInfo", off: 0, size: 4, ty: "AEEUMTSBearerTechInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEBitmapInfo", size: 12, align: 4, packed: false, declared_in: "AEEIBitmap.h",
        fields: &[FieldInfo { name: "cx", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nDepth", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEECDMABearerTechInfo", size: 4, align: 4, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "uCDMABearerTech", off: 0, size: 4, ty: "AEECDMABearerTech", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEECallHistoryEntry", size: 8, align: 4, packed: false, declared_in: "AEECallHistory.h",
        fields: &[FieldInfo { name: "pFields", off: 0, size: 4, ty: "AEECallHistoryField*", bits: 0, bit_off: 0 }, FieldInfo { name: "wNumFields", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEECallHistoryField", size: 12, align: 4, packed: false, declared_in: "AEECallHistory.h",
        fields: &[FieldInfo { name: "ClsId", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wID", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wDataLen", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 8, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEECameraNotify", size: 20, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "pCam", off: 0, size: 4, ty: "ICamera*", bits: 0, bit_off: 0 }, FieldInfo { name: "nCmd", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nSubCmd", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nStatus", off: 8, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nReserved", off: 10, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 12, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSize", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEECardInfo", size: 16, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "pszSerialNum", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "nSerialNumSize", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pszModelNum", off: 8, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "nModelNumSize", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEECipherInit", size: 32, align: 4, packed: false, declared_in: "AEESecurity.h",
        fields: &[FieldInfo { name: "pbIV", off: 0, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "cbIV", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pbKey1", off: 8, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "cbKey1", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pbKey2", off: 16, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "cbKey2", off: 20, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pbKey3", off: 24, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "cbKey3", off: 28, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEECircle", size: 6, align: 2, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "cx", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "r", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEClip", size: 16, align: 4, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "type", off: 0, size: 4, ty: "AEEClipShape", bits: 0, bit_off: 0 }, FieldInfo { name: "shape", off: 4, size: 12, ty: "AEEClip@1", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEClip@1", size: 12, align: 4, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "rect", off: 0, size: 8, ty: "AEERect", bits: 0, bit_off: 0 }, FieldInfo { name: "circle", off: 0, size: 6, ty: "AEECircle", bits: 0, bit_off: 0 }, FieldInfo { name: "ellipse", off: 0, size: 8, ty: "AEEEllipse", bits: 0, bit_off: 0 }, FieldInfo { name: "pie", off: 0, size: 10, ty: "AEEPie", bits: 0, bit_off: 0 }, FieldInfo { name: "triangle", off: 0, size: 12, ty: "AEETriangle", bits: 0, bit_off: 0 }, FieldInfo { name: "polygon", off: 0, size: 8, ty: "AEEPolygon", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEECondVar", size: 4, align: 4, packed: false, declared_in: "AEEThreadUtil.h",
        fields: &[FieldInfo { name: "pcbWaiting", off: 0, size: 4, ty: "AEECallback*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDBComplex", size: 6, align: 2, packed: false, declared_in: "AEEDB.h",
        fields: &[FieldInfo { name: "wNumSubfields", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "byVersion", off: 2, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "byReserved", off: 3, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "buf", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDBField", size: 16, align: 4, packed: false, declared_in: "AEEDB.h",
        fields: &[FieldInfo { name: "fType", off: 0, size: 4, ty: "AEEDBFieldType", bits: 0, bit_off: 0 }, FieldInfo { name: "fName", off: 4, size: 4, ty: "AEEDBFieldName", bits: 0, bit_off: 0 }, FieldInfo { name: "wDataLen", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "pBuffer", off: 12, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDBSubfield", size: 6, align: 2, packed: false, declared_in: "AEEDB.h",
        fields: &[FieldInfo { name: "fID", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wDataLen", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "fType", off: 4, size: 1, ty: "int8", bits: 0, bit_off: 0 }, FieldInfo { name: "buf", off: 5, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDIB", size: 32, align: 4, packed: false, declared_in: "AEEBitmap.h",
        fields: &[FieldInfo { name: "pPaletteMap", off: 0, size: 4, ty: "IQueryInterface*", bits: 0, bit_off: 0 }, FieldInfo { name: "pBmp", off: 4, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "pRGB", off: 8, size: 4, ty: "uint32*", bits: 0, bit_off: 0 }, FieldInfo { name: "transparentColor", off: 12, size: 4, ty: "NativeColor", bits: 0, bit_off: 0 }, FieldInfo { name: "cx", off: 16, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 18, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nPitch", off: 20, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "cntRGB", off: 22, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "colorDepth", off: 24, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "reserved", off: 25, size: 7, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDNSItem", size: 20, align: 4, packed: false, declared_in: "AEEIDNS.h",
        fields: &[FieldInfo { name: "pbyDomain", off: 0, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "nType", off: 4, size: 2, ty: "AEEDNSType", bits: 0, bit_off: 0 }, FieldInfo { name: "nClass", off: 6, size: 2, ty: "AEEDNSClass", bits: 0, bit_off: 0 }, FieldInfo { name: "nTTL", off: 8, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "pbyData", off: 12, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "cbData", off: 16, size: 4, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDNSResponse", size: 28, align: 4, packed: false, declared_in: "AEEIDNS.h",
        fields: &[FieldInfo { name: "uFlags", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "uReserved", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nQuestions", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nAnswers", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nServers", off: 8, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nAdditional", off: 10, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "pQuestions", off: 12, size: 4, ty: "AEEDNSItem*", bits: 0, bit_off: 0 }, FieldInfo { name: "pAnswers", off: 16, size: 4, ty: "AEEDNSItem*", bits: 0, bit_off: 0 }, FieldInfo { name: "pServers", off: 20, size: 4, ty: "AEEDNSItem*", bits: 0, bit_off: 0 }, FieldInfo { name: "pAdditional", off: 24, size: 4, ty: "AEEDNSItem*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDNSResult", size: 20, align: 4, packed: false, declared_in: "AEENetworkTypes.h",
        fields: &[FieldInfo { name: "nResult", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "addrs", off: 4, size: 16, ty: "INAddr", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDateTime", size: 14, align: 2, packed: false, declared_in: "AEEDateTime.h",
        fields: &[FieldInfo { name: "wYear", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wMonth", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wDay", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wHour", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wMinute", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wSecond", off: 10, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wWeekDay", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDeviceInfo", size: 64, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "cxScreen", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "cyScreen", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "cxAltScreen", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "cyAltScreen", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "cxScrollBar", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wEncoding", off: 10, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wMenuTextScroll", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nColorDepth", off: 14, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "unused2", off: 16, size: 4, ty: "EmptyEnum", bits: 0, bit_off: 0 }, FieldInfo { name: "wMenuImageDelay", off: 20, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwRAM", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "bAltDisplay", off: 28, size: 4, ty: "flg", bits: 1, bit_off: 0 }, FieldInfo { name: "bFlip", off: 28, size: 4, ty: "flg", bits: 1, bit_off: 1 }, FieldInfo { name: "bVibrator", off: 28, size: 4, ty: "flg", bits: 1, bit_off: 2 }, FieldInfo { name: "bExtSpeaker", off: 28, size: 4, ty: "flg", bits: 1, bit_off: 3 }, FieldInfo { name: "bVR", off: 28, size: 4, ty: "flg", bits: 1, bit_off: 4 }, FieldInfo { name: "bPosLoc", off: 28, size: 4, ty: "flg", bits: 1, bit_off: 5 }, FieldInfo { name: "bMIDI", off: 28, size: 4, ty: "flg", bits: 1, bit_off: 6 }, FieldInfo { name: "bCMX", off: 28, size: 4, ty: "flg", bits: 1, bit_off: 7 }, FieldInfo { name: "bPen", off: 28, size: 4, ty: "flg", bits: 1, bit_off: 8 }, FieldInfo { name: "dwPromptProps", off: 32, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wKeyCloseApp", off: 36, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wKeyCloseAllApps", off: 38, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwLang", off: 40, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wStructSize", off: 44, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwNetLinger", off: 48, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSleepDefer", off: 52, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wMaxPath", off: 56, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwPlatformID", off: 60, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEDeviceNotify", size: 8, align: 4, packed: false, declared_in: "AEEDeviceNotifier.h",
        fields: &[FieldInfo { name: "wParam", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwParam", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEEGLCompositeSurfaceCaps", size: 24, align: 4, packed: false, declared_in: "AEEEGLTypes.h",
        fields: &[FieldInfo { name: "max_overlay", off: 0, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "max_underlay", off: 4, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "max_surface_per_layer", off: 8, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "max_total_surfaces", off: 12, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "pbuffer_support", off: 16, size: 4, ty: "AEEEGLBoolean", bits: 0, bit_off: 0 }, FieldInfo { name: "pixmap_support", off: 20, size: 4, ty: "AEEEGLBoolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEEGLSurfaceScaleCaps", size: 48, align: 4, packed: false, declared_in: "AEEEGLTypes.h",
        fields: &[FieldInfo { name: "MinXScaleFactor", off: 0, size: 4, ty: "AEEEGLfixed", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxXScaleFactor", off: 4, size: 4, ty: "AEEEGLfixed", bits: 0, bit_off: 0 }, FieldInfo { name: "MinYScaleFactor", off: 8, size: 4, ty: "AEEEGLfixed", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxYScaleFactor", off: 12, size: 4, ty: "AEEEGLfixed", bits: 0, bit_off: 0 }, FieldInfo { name: "MinSrcWidth", off: 16, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxSrcWidth", off: 20, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MinSrcHeight", off: 24, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxSrcHeight", off: 28, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MinDstWidth", off: 32, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxDstWidth", off: 36, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MinDstHeight", off: 40, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxDstHeight", off: 44, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEEGLSurfaceScaleRect", size: 16, align: 4, packed: false, declared_in: "AEEEGLTypes.h",
        fields: &[FieldInfo { name: "x", off: 0, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "y", off: 4, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "width", off: 8, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "height", off: 12, size: 4, ty: "AEEEGLint", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEEllipse", size: 8, align: 2, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "cx", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "wx", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "wy", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEExtent", size: 8, align: 4, packed: false, declared_in: "AEEExtent.h",
        fields: &[FieldInfo { name: "cx", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEFSFreeSpace", size: 16, align: 8, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "qwTotalSpace", off: 0, size: 8, ty: "uint64", bits: 0, bit_off: 0 }, FieldInfo { name: "qwFreeSpace", off: 8, size: 8, ty: "uint64", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEFileInfo", size: 76, align: 4, packed: false, declared_in: "AEEFile.h",
        fields: &[FieldInfo { name: "attrib", off: 0, size: 1, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "dwCreationDate", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSize", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "szName", off: 12, size: 64, ty: "char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEFileInfoEx", size: 40, align: 4, packed: false, declared_in: "AEEFile.h",
        fields: &[FieldInfo { name: "nStructSize", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "attrib", off: 4, size: 1, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "dwCreationDate", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSize", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pszFile", off: 16, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "nMaxFile", off: 20, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pszDescription", off: 24, size: 4, ty: "AECHAR*", bits: 0, bit_off: 0 }, FieldInfo { name: "nDescriptionSize", off: 28, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pClasses", off: 32, size: 4, ty: "uint32*", bits: 0, bit_off: 0 }, FieldInfo { name: "nClassesSize", off: 36, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEFileUseInfo", size: 12, align: 4, packed: false, declared_in: "AEEFile.h",
        fields: &[FieldInfo { name: "wMaxFiles", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wFilesUsed", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwMaxSpace", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSpaceUsed", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEFlipInfoEx", size: 16, align: 4, packed: false, declared_in: "AEEFlip.h",
        fields: &[FieldInfo { name: "dwStructSize", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "flipInfo", off: 4, size: 4, ty: "AEEFlipInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "flipCapability", off: 8, size: 4, ty: "AEEFlipCapability", bits: 0, bit_off: 0 }, FieldInfo { name: "flipPositionType", off: 12, size: 4, ty: "AEEFlipPositionType", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEFontInfo", size: 4, align: 2, packed: false, declared_in: "AEEIFont.h",
        fields: &[FieldInfo { name: "nAscent", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nDescent", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEGPSConfig", size: 20, align: 4, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "mode", off: 0, size: 2, ty: "AEEGPSMode", bits: 0, bit_off: 0 }, FieldInfo { name: "nFixes", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nInterval", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "optim", off: 6, size: 1, ty: "AEEGPSOpt", bits: 0, bit_off: 0 }, FieldInfo { name: "qos", off: 7, size: 1, ty: "AEEGPSQos", bits: 0, bit_off: 0 }, FieldInfo { name: "server", off: 8, size: 12, ty: "AEEGPSServer", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEGPSIPServer", size: 8, align: 4, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "addr", off: 0, size: 4, ty: "INAddr", bits: 0, bit_off: 0 }, FieldInfo { name: "port", off: 4, size: 2, ty: "INPort", bits: 0, bit_off: 0 }, FieldInfo { name: "reserved", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEGPSInfo", size: 44, align: 4, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "dwTimeStamp", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "status", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwLat", off: 8, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwLon", off: 12, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "wAltitude", off: 16, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "wHeading", off: 18, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wVelocityHor", off: 20, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "bVelocityVer", off: 22, size: 1, ty: "int8", bits: 0, bit_off: 0 }, FieldInfo { name: "accuracy", off: 23, size: 1, ty: "AEEGPSAccuracy", bits: 0, bit_off: 0 }, FieldInfo { name: "fValid", off: 24, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "bHorUnc", off: 26, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "bHorUncAngle", off: 27, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "bHorUncPerp", off: 28, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "Version", off: 29, size: 1, ty: "AEEGPSInfoVersion", bits: 0, bit_off: 0 }, FieldInfo { name: "wVerUnc", off: 30, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "GPS_UTCOffset", off: 32, size: 1, ty: "int8", bits: 0, bit_off: 0 }, FieldInfo { name: "LocProbability", off: 33, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "method", off: 36, size: 4, ty: "AEEGPSMethod", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTimeMS", off: 40, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEGPSServer", size: 12, align: 4, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "svrType", off: 0, size: 4, ty: "AEEGPSSvrType", bits: 0, bit_off: 0 }, FieldInfo { name: "svr", off: 4, size: 8, ty: "AEEGPSServer@1", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEGPSServer@1", size: 8, align: 4, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "ipsvr", off: 0, size: 8, ty: "AEEGPSIPServer", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEHIDButtonInfo", size: 20, align: 4, packed: false, declared_in: "AEEIHIDDevice.h",
        fields: &[FieldInfo { name: "nButtonID", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nState", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nButtonUID", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nButtonMin", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nButtonMax", off: 16, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEHIDDeviceInfo", size: 12, align: 4, packed: false, declared_in: "AEEIHIDDevice.h",
        fields: &[FieldInfo { name: "nDeviceType", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "wProductID", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wVendorID", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "bBluetoothDevice", off: 8, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEHIDPositionInfo", size: 100, align: 4, packed: false, declared_in: "AEEIHIDDevice.h",
        fields: &[FieldInfo { name: "bRelativeAxes", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "nX", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nY", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nZ", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nRx", off: 16, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nRy", off: 20, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nRz", off: 24, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nVX", off: 28, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nVY", off: 32, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nVZ", off: 36, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nVRx", off: 40, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nVRy", off: 44, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nVRz", off: 48, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nAX", off: 52, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nAY", off: 56, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nAZ", off: 60, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nARx", off: 64, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nARy", off: 68, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nARz", off: 72, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nFX", off: 76, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nFY", off: 80, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nFZ", off: 84, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nFRx", off: 88, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nFRy", off: 92, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nFRz", off: 96, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEHIDThumbstickValues", size: 8, align: 2, packed: false, declared_in: "AEEHIDThumbsticks.h",
        fields: &[FieldInfo { name: "sLeftX", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "sLeftY", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "sRightX", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "sRightY", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEHWIDType", size: 8, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "uLength", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "bHWIDLength", off: 2, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "bHWIDType", off: 3, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "HWID", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEHelperFuncs", size: 468, align: 4, packed: false, declared_in: "AEEStdLib.h",
        fields: &[FieldInfo { name: "memmove", off: 0, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "memset", off: 4, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strcpy", off: 8, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strcat", off: 12, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strcmp", off: 16, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strlen", off: 20, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strchr", off: 24, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strrchr", off: 28, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "sprintf", off: 32, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrcpy", off: 36, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrcat", off: 40, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrcmp", off: 44, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrlen", off: 48, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrchr", off: 52, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrrchr", off: 56, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wsprintf", off: 60, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strtowstr", off: 64, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrtostr", off: 68, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrtofloat", off: 72, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "floattowstr", off: 76, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "utf8towstr", off: 80, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrtoutf8", off: 84, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrlower", off: 88, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrupper", off: 92, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "chartype", off: 96, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "SetupNativeImage", off: 100, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "malloc", off: 104, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "free", off: 108, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrdup", off: 112, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "realloc", off: 116, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wwritelongex", off: 120, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrsize", off: 124, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrncopyn", off: 128, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "OEMStrLen", off: 132, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "OEMStrSize", off: 136, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "GetAEEVersion", off: 140, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "atoi", off: 144, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "f_op", off: 148, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "f_cmp", off: 152, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "dbgprintf", off: 156, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrcompress", off: 160, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_LocalTimeOffset", off: 164, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_GetRand", off: 168, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_GetTimeMS", off: 172, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_GetUpTimeMS", off: 176, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_GetSeconds", off: 180, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_GetJulianDate", off: 184, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "sysfree", off: 188, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "GetAppInstance", off: 192, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strtoul", off: 196, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strncpy", off: 200, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strncmp", off: 204, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "stricmp", off: 208, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strnicmp", off: 212, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strstr", off: 216, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "memcmp", off: 220, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "memchr", off: 224, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strexpand", off: 228, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "stristr", off: 232, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "memstr", off: 236, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrncmp", off: 240, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strdup", off: 244, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strbegins", off: 248, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strends", off: 252, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strchrend", off: 256, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strchrsend", off: 260, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "memrchr", off: 264, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "memchrend", off: 268, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "memrchrbegin", off: 272, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strlower", off: 276, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strupper", off: 280, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstricmp", off: 284, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrnicmp", off: 288, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "inet_aton", off: 292, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "inet_ntoa", off: 296, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "swapl", off: 300, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "swaps", off: 304, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "GetFSFree", off: 308, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "GetRAMFree", off: 312, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "vsprintf", off: 316, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "vsnprintf", off: 320, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "snprintf", off: 324, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_JulianToSeconds", off: 328, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strlcpy", off: 332, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strlcat", off: 336, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrlcpy", off: 340, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wstrlcat", off: 344, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "setstaticptr", off: 348, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "f_assignstr", off: 352, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "f_assignint", off: 356, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wwritelong", off: 360, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "dbgheapmark", off: 364, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "lockmem", off: 368, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "unlockmem", off: 372, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "dumpheap", off: 376, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "strtod", off: 380, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "f_calc", off: 384, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "sleep", off: 388, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "getlasterror", off: 392, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "wgs84_to_degrees", off: 396, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "dbgevent", off: 400, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_IsBadPtr", off: 404, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_basename", off: 408, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_makepath", off: 412, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_splitpath", off: 416, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_stribegins", off: 420, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "aee_GetUTCSeconds", off: 424, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "f_toint", off: 428, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "f_get", off: 432, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "qsort", off: 436, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "trunc", off: 440, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "utrunc", off: 444, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "err_realloc", off: 448, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "err_strdup", off: 452, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "inet_pton", off: 456, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "inet_ntop", off: 460, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }, FieldInfo { name: "GetALSContext", off: 464, size: 4, ty: "fnptr", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEICMPErrMsg", size: 148, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "ee", off: 0, size: 16, ty: "AEESockExtendedErr", bits: 0, bit_off: 0 }, FieldInfo { name: "sas", off: 16, size: 130, ty: "AEESockAddrStorage", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEIPv6PrefixInfo", size: 4, align: 2, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "uState", off: 0, size: 2, ty: "AEEIPv6PrefixState", bits: 0, bit_off: 0 }, FieldInfo { name: "uPrefixLen", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEIPv6PrivAddrInfo", size: 24, align: 4, packed: false, declared_in: "AEENetAddrTypes.h",
        fields: &[FieldInfo { name: "uState", off: 0, size: 4, ty: "AEEIPv6PrivAddrState", bits: 0, bit_off: 0 }, FieldInfo { name: "bIsUnique", off: 4, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "ipaAddr", off: 8, size: 16, ty: "INAddr6", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEImageExtFunc", size: 12, align: 4, packed: false, declared_in: "AEEIImage.h",
        fields: &[FieldInfo { name: "id", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pBuf", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "nBufSize", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEImageInfo", size: 10, align: 2, packed: false, declared_in: "AEEImageInfo.h",
        fields: &[FieldInfo { name: "cx", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nColors", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "bAnimated", off: 6, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "cxFrame", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEItemStyle", size: 12, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "ft", off: 0, size: 4, ty: "AEEFrameType", bits: 0, bit_off: 0 }, FieldInfo { name: "xOffset", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "yOffset", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "roImage", off: 8, size: 4, ty: "AEERasterOp", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEKeyInfoType", size: 16, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "wKey", off: 0, size: 4, ty: "AVKType", bits: 0, bit_off: 0 }, FieldInfo { name: "flag", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 8, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnDataSize", off: 12, size: 4, ty: "uint32*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEELine", size: 8, align: 2, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "sx", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "sy", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "ex", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "ey", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEELinger", size: 8, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "bOn", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "dwLinger", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMcastSessionJoinMBMS", size: 8, align: 4, packed: false, declared_in: "AEEIMcastSession_JoinMBMS.h",
        fields: &[FieldInfo { name: "pGroupAddr", off: 0, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "pMBMSJoinInfo", off: 4, size: 4, ty: "AEEMcast_MBMSJoinInfo*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMcast_MBMSJoinInfo", size: 32, align: 4, packed: false, declared_in: "AEEMcastTypes_MBMS.h",
        fields: &[FieldInfo { name: "tmgi", off: 0, size: 8, ty: "uint64struct", bits: 0, bit_off: 0 }, FieldInfo { name: "session_start_time", off: 8, size: 8, ty: "uint64struct", bits: 0, bit_off: 0 }, FieldInfo { name: "session_end_time", off: 16, size: 8, ty: "uint64struct", bits: 0, bit_off: 0 }, FieldInfo { name: "priority", off: 24, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "service_method", off: 26, size: 2, ty: "AEEMcast_MBMSServiceMethod", bits: 0, bit_off: 0 }, FieldInfo { name: "service_type", off: 28, size: 2, ty: "AEEMcast_MBMSServiceType", bits: 0, bit_off: 0 }, FieldInfo { name: "selected_service", off: 30, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "service_security", off: 31, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaAudioSync", size: 16, align: 8, packed: false, declared_in: "AEEIMedia.h",
        fields: &[FieldInfo { name: "ullTimeStampMS", off: 0, size: 8, ty: "uint64", bits: 0, bit_off: 0 }, FieldInfo { name: "ullSamples", off: 8, size: 8, ty: "uint64", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaCmdNotify", size: 28, align: 4, packed: false, declared_in: "AEEIMedia.h",
        fields: &[FieldInfo { name: "clsMedia", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pIMedia", off: 4, size: 4, ty: "IMedia*", bits: 0, bit_off: 0 }, FieldInfo { name: "nCmd", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nSubCmd", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nStatus", off: 16, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pCmdData", off: 20, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSize", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaCreateInfo", size: 20, align: 4, packed: false, declared_in: "AEEMediaUtil.h",
        fields: &[FieldInfo { name: "dwStructSize", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nState", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nCount", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pmdList", off: 12, size: 4, ty: "AEEMediaDataEx*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnCapsList", off: 16, size: 4, ty: "int*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaData", size: 12, align: 4, packed: false, declared_in: "AEEIMedia.h",
        fields: &[FieldInfo { name: "clsData", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSize", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaDataEx", size: 36, align: 4, packed: false, declared_in: "AEEIMedia.h",
        fields: &[FieldInfo { name: "clsData", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSize", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwStructSize", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwCaps", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "bRaw", off: 20, size: 4, ty: "unsigned", bits: 1, bit_off: 0 }, FieldInfo { name: "pSpec", off: 24, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSpecSize", off: 28, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBufferSize", off: 32, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaDecryptInfo", size: 24, align: 4, packed: false, declared_in: "AEEMediaDecrypt.h",
        fields: &[FieldInfo { name: "wStructSize", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwFormat", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwType", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pKey", off: 12, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "wKeyLen", off: 16, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTxID", off: 20, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaEncodeInfo", size: 20, align: 4, packed: false, declared_in: "AEEMediaUtil.h",
        fields: &[FieldInfo { name: "pmdDest", off: 0, size: 4, ty: "AEEMediaData*", bits: 0, bit_off: 0 }, FieldInfo { name: "pmdList", off: 4, size: 4, ty: "AEEMediaData*", bits: 0, bit_off: 0 }, FieldInfo { name: "nCount", off: 8, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "pEncodeSpec", off: 12, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSpecSize", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaEncodeResult", size: 20, align: 4, packed: false, declared_in: "AEEMediaUtil.h",
        fields: &[FieldInfo { name: "nResult", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "clsMedia", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pmdDest", off: 8, size: 4, ty: "AEEMediaData*", bits: 0, bit_off: 0 }, FieldInfo { name: "pEncodeSpec", off: 12, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSpecSize", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaMIDISpec", size: 6, align: 2, packed: false, declared_in: "AEEMediaFormats.h",
        fields: &[FieldInfo { name: "nFormat", off: 0, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "nTracks", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nDivision", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaMP3Spec", size: 144, align: 4, packed: false, declared_in: "AEEMediaFormats.h",
        fields: &[FieldInfo { name: "nVersion", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nLayer", off: 4, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "bCRCFlag", off: 5, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "wBitRate", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSampleRate", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nChannel", off: 12, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "nExtension", off: 13, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "bCopyrightFlag", off: 14, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bOriginalFlag", off: 15, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "nEmphasis", off: 16, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "szTitle", off: 17, size: 30, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "szArtist", off: 47, size: 30, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "szAlbum", off: 77, size: 30, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "szYear", off: 107, size: 4, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "szComment", off: 111, size: 30, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "nGenre", off: 141, size: 1, ty: "byte", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaSVGSpec", size: 144, align: 4, packed: false, declared_in: "AEEMediaSVG.h",
        fields: &[FieldInfo { name: "flgFeatures", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nColorScheme", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "szTitle", off: 4, size: 128, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "nFrameWidth", off: 132, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nFrameHeight", off: 136, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nStructSize", off: 140, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMediaWaveSpec", size: 36, align: 4, packed: false, declared_in: "AEEMediaFormats.h",
        fields: &[FieldInfo { name: "wSize", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "clsMedia", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wChannels", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSamplesPerSec", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wBitsPerSample", off: 16, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "bUnsigned", off: 20, size: 4, ty: "flg", bits: 1, bit_off: 0 }, FieldInfo { name: "dwBytesPerSec", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wBlockSize", off: 28, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwExtra", off: 32, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMenuColors", size: 40, align: 4, packed: false, declared_in: "AEEMenu.h",
        fields: &[FieldInfo { name: "wMask", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "cBack", off: 4, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }, FieldInfo { name: "cText", off: 8, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }, FieldInfo { name: "cSelBack", off: 12, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }, FieldInfo { name: "cSelText", off: 16, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }, FieldInfo { name: "cFrame", off: 20, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }, FieldInfo { name: "cScrollbar", off: 24, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }, FieldInfo { name: "cScrollbarFill", off: 28, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }, FieldInfo { name: "cTitle", off: 32, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }, FieldInfo { name: "cTitleText", off: 36, size: 4, ty: "RGBVAL", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMod", size: 20, align: 4, packed: false, declared_in: "AEEModGen.h",
        fields: &[FieldInfo { name: "vtIModule", off: 0, size: 4, ty: "IModule", bits: 0, bit_off: 0 }, FieldInfo { name: "m_nRefs", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "m_pIShell", off: 8, size: 4, ty: "IShell*", bits: 0, bit_off: 0 }, FieldInfo { name: "pfnModCrInst", off: 12, size: 4, ty: "PFNMODCREATEINST", bits: 0, bit_off: 0 }, FieldInfo { name: "pfnModFreeData", off: 16, size: 4, ty: "PFNFREEMODDATA", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEMutex", size: 8, align: 4, packed: false, declared_in: "AEEThreadUtil.h",
        fields: &[FieldInfo { name: "pcbWaiting", off: 0, size: 4, ty: "AEECallback*", bits: 0, bit_off: 0 }, FieldInfo { name: "pitHolding", off: 4, size: 4, ty: "IThread*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEENetDomainName", size: 256, align: 1, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "domain_name", off: 0, size: 256, ty: "char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEENetStats", size: 32, align: 4, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "dwOpenTime", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwActiveTime", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBytes", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwRate", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTotalOpenTime", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTotalActiveTime", off: 20, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTotalBytes", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTotalRate", off: 28, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEENetUsage", size: 16, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "dwTCPSent", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTCPRecv", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwUDPSent", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwUDPRecv", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEENetworkPrefixInfo", size: 20, align: 4, packed: false, declared_in: "AEEINetwork.h",
        fields: &[FieldInfo { name: "addr", off: 0, size: 16, ty: "INAddr6", bits: 0, bit_off: 0 }, FieldInfo { name: "prefixInfo", off: 16, size: 4, ty: "AEEIPv6PrefixInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEENetworkQoSProfiles", size: 12, align: 4, packed: false, declared_in: "AEEINetwork.h",
        fields: &[FieldInfo { name: "awQoSProfiles", off: 0, size: 4, ty: "uint16*", bits: 0, bit_off: 0 }, FieldInfo { name: "uNumProfiles", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "puNumProfilesRequired", off: 8, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEENetworkSipServerAddresses", size: 12, align: 4, packed: false, declared_in: "AEEINetwork.h",
        fields: &[FieldInfo { name: "aSipServerAddresses", off: 0, size: 4, ty: "IPAddr*", bits: 0, bit_off: 0 }, FieldInfo { name: "uNumAddresses", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "puNumAddressesRequired", off: 8, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEENetworkSipServerDomainNames", size: 12, align: 4, packed: false, declared_in: "AEEINetwork.h",
        fields: &[FieldInfo { name: "aSipServerDomainNames", off: 0, size: 4, ty: "AEENetDomainName*", bits: 0, bit_off: 0 }, FieldInfo { name: "uNumNames", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "puNumNamesRequired", off: 8, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEENotify", size: 20, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "cls", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pNotifier", off: 4, size: 4, ty: "INotifier*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwMask", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 12, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "st", off: 16, size: 4, ty: "AEENotifyStatus", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEENotifyFailure", size: 12, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "cls", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwMask", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nError", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEOBEXAuthResp", size: 24, align: 4, packed: false, declared_in: "AEEOBEXComDefs.h",
        fields: &[FieldInfo { name: "nUidLen", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "aUserId", off: 4, size: 20, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEOBEXEvent", size: 12, align: 4, packed: false, declared_in: "AEEIOBEXSrvSession.h",
        fields: &[FieldInfo { name: "packet", off: 0, size: 4, ty: "AEEOBEXPacket", bits: 0, bit_off: 0 }, FieldInfo { name: "flags", off: 4, size: 4, ty: "AEEOBEXFlags", bits: 0, bit_off: 0 }, FieldInfo { name: "cmdStatus", off: 8, size: 4, ty: "AEEOBEXCmdStatus", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEOBEXHeader", size: 12, align: 4, packed: false, declared_in: "AEEIOBEXHeader.h",
        fields: &[FieldInfo { name: "headerId", off: 0, size: 4, ty: "AEEOBEXHeaderID", bits: 0, bit_off: 0 }, FieldInfo { name: "headerType", off: 4, size: 4, ty: "AEEOBEXHeaderType", bits: 0, bit_off: 0 }, FieldInfo { name: "nHeaderSize", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEODCBData", size: 20, align: 4, packed: false, declared_in: "AEEMenu.h",
        fields: &[FieldInfo { name: "nSize", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nAction", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nActionInfo", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "rc", off: 8, size: 8, ty: "AEERect", bits: 0, bit_off: 0 }, FieldInfo { name: "pIDisplay", off: 16, size: 4, ty: "IDisplay*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEOrientationAzimuthInfo", size: 8, align: 4, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "wSize", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wAzimuth", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTimeStamp", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEOutageInfo", size: 12, align: 4, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "nStatus", off: 0, size: 4, ty: "AEEOutageStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "nTimeToOutage", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "uDuration", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEPDPInfo", size: 1, align: 1, packed: false, declared_in: "AEEINetUtils.h",
        fields: &[FieldInfo { name: "bValid", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEParmInfo", size: 20, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "nMin", off: 0, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "nMax", off: 4, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "nStep", off: 8, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "nDefault", off: 12, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "nCurrent", off: 16, size: 4, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEPie", size: 10, align: 2, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "cx", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "r", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "startAngle", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "arcAngle", off: 8, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEPoint", size: 4, align: 2, packed: false, declared_in: "AEEPoint.h",
        fields: &[FieldInfo { name: "x", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "y", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEPolygon", size: 8, align: 4, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "len", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "points", off: 4, size: 4, ty: "AEEPoint*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEPolyline", size: 8, align: 4, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "len", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "points", off: 4, size: 4, ty: "AEEPoint*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEPositionInfo", size: 12, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "dwLat", off: 0, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwLon", off: 4, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTimeStamp", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEPositionInfoEx", size: 96, align: 8, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "dwSize", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "fLatitude", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 0 }, FieldInfo { name: "fLongitude", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 1 }, FieldInfo { name: "fAltitude", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 2 }, FieldInfo { name: "fHeading", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 3 }, FieldInfo { name: "fHorVelocity", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 4 }, FieldInfo { name: "fVerVelocity", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 5 }, FieldInfo { name: "fHorUnc", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 6 }, FieldInfo { name: "fAltitudeUnc", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 7 }, FieldInfo { name: "fUTCOffsetValid", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 8 }, FieldInfo { name: "Latitude", off: 8, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "Longitude", off: 16, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "nAltitude", off: 24, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "Heading", off: 32, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "HorVelocity", off: 40, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "VerVelocity", off: 48, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "HorUncAngle", off: 56, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "HorUnc", off: 64, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "HorUncPerp", off: 72, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "AltitudeUnc", off: 80, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "GPS_UTCOffset", off: 88, size: 1, ty: "int8", bits: 0, bit_off: 0 }, FieldInfo { name: "LocProbability", off: 89, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "method", off: 92, size: 4, ty: "AEEGPSMethod", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEPositionInfoEx_ALIGN_32", size: 92, align: 4, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "dwSize", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "fLatitude", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 0 }, FieldInfo { name: "fLongitude", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 1 }, FieldInfo { name: "fAltitude", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 2 }, FieldInfo { name: "fHeading", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 3 }, FieldInfo { name: "fHorVelocity", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 4 }, FieldInfo { name: "fVerVelocity", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 5 }, FieldInfo { name: "fHorUnc", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 6 }, FieldInfo { name: "fAltitudeUnc", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 7 }, FieldInfo { name: "fUTCOffsetValid", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 8 }, FieldInfo { name: "Latitude", off: 8, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "Longitude", off: 16, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "nAltitude", off: 24, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "Heading", off: 28, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "HorVelocity", off: 36, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "VerVelocity", off: 44, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "HorUncAngle", off: 52, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "HorUnc", off: 60, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "HorUncPerp", off: 68, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "AltitudeUnc", off: 76, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "GPS_UTCOffset", off: 84, size: 1, ty: "int8", bits: 0, bit_off: 0 }, FieldInfo { name: "LocProbability", off: 85, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "method", off: 88, size: 4, ty: "AEEGPSMethod", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEPromptInfo", size: 40, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "pszRes", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "pTitle", off: 4, size: 4, ty: "AECHAR*", bits: 0, bit_off: 0 }, FieldInfo { name: "pText", off: 8, size: 4, ty: "AECHAR*", bits: 0, bit_off: 0 }, FieldInfo { name: "wTitleID", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wTextID", off: 14, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wDefBtn", off: 16, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "pBtnIDs", off: 20, size: 4, ty: "uint16*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwProps", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "fntTitle", off: 28, size: 4, ty: "AEEFont", bits: 0, bit_off: 0 }, FieldInfo { name: "fntText", off: 32, size: 4, ty: "AEEFont", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTimeout", off: 36, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQOSFILTER_IPv4Addr", size: 8, align: 4, packed: false, declared_in: "AEEIQoSFilter.h",
        fields: &[FieldInfo { name: "addr", off: 0, size: 4, ty: "INAddr", bits: 0, bit_off: 0 }, FieldInfo { name: "subnet_mask", off: 4, size: 4, ty: "INAddr", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQOSFILTER_IPv4TOS", size: 2, align: 1, packed: false, declared_in: "AEEIQoSFilter.h",
        fields: &[FieldInfo { name: "val", off: 0, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "mask", off: 1, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQOSFILTER_Port", size: 4, align: 2, packed: false, declared_in: "AEEIQoSFilter.h",
        fields: &[FieldInfo { name: "port", off: 0, size: 2, ty: "INPort", bits: 0, bit_off: 0 }, FieldInfo { name: "range", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQOSFLOW_DataRateMinMax", size: 8, align: 4, packed: false, declared_in: "AEEIQoSFlow.h",
        fields: &[FieldInfo { name: "max_rate", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "guaranteed_rate", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQOSFLOW_DataRateTokenBucket", size: 12, align: 4, packed: false, declared_in: "AEEIQoSFlow.h",
        fields: &[FieldInfo { name: "peak_rate", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "token_rate", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "size", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQOSFLOW_NominalSDUSize", size: 8, align: 4, packed: false, declared_in: "AEEIQoSFlow.h",
        fields: &[FieldInfo { name: "is_fixed", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "size", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQOSFLOW_PacketErrorRate", size: 4, align: 2, packed: false, declared_in: "AEEIQoSFlow.h",
        fields: &[FieldInfo { name: "multiplier", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "exponent", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQoSBundleOpenEx", size: 12, align: 4, packed: false, declared_in: "AEEIQoSBundle.h",
        fields: &[FieldInfo { name: "pIQoSSessions", off: 0, size: 4, ty: "IQoSList*", bits: 0, bit_off: 0 }, FieldInfo { name: "pIQoSSpecs", off: 4, size: 4, ty: "IQoSList*", bits: 0, bit_off: 0 }, FieldInfo { name: "opcode", off: 8, size: 4, ty: "AEEQoSBundleRequestOpcode", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQoSError", size: 20, align: 4, packed: false, declared_in: "AEEIQoSSpec.h",
        fields: &[FieldInfo { name: "eErrorCode", off: 0, size: 4, ty: "AEEQoSErrorCode", bits: 0, bit_off: 0 }, FieldInfo { name: "eField", off: 4, size: 4, ty: "AEEQoSErrorField", bits: 0, bit_off: 0 }, FieldInfo { name: "nIndex", off: 8, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "nOption", off: 12, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "ext", off: 16, size: 4, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQoSSessionError", size: 8, align: 4, packed: false, declared_in: "AEEIQoSBundle.h",
        fields: &[FieldInfo { name: "nErrorCode", off: 0, size: 4, ty: "AEEQoSSessionErrorCode", bits: 0, bit_off: 0 }, FieldInfo { name: "nIndex", off: 4, size: 4, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQoSSessionGetSpecDuringAnyState", size: 8, align: 4, packed: false, declared_in: "AEEIQoSSession.h",
        fields: &[FieldInfo { name: "ppRxFlowSpec", off: 0, size: 4, ty: "IQoSFlow**", bits: 0, bit_off: 0 }, FieldInfo { name: "ppTxFlowSpec", off: 4, size: 4, ty: "IQoSFlow**", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQosSesssionModifyEx", size: 8, align: 4, packed: false, declared_in: "AEEIQoSSession.h",
        fields: &[FieldInfo { name: "piRequestedQoSSpec", off: 0, size: 4, ty: "IQoSSpec*", bits: 0, bit_off: 0 }, FieldInfo { name: "uQoSModifyMask", off: 4, size: 2, ty: "AEEQoSModifyMask", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEQosSesssionSelectNetworkEx", size: 8, align: 4, packed: false, declared_in: "AEEIQoSSession.h",
        fields: &[FieldInfo { name: "nNetwork", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "sFamily", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEERLP3Cfg", size: 8, align: 1, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "ucFwdNakRounds", off: 0, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "aucFwdNaksPerRound", off: 1, size: 3, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "ucRevNakRounds", off: 4, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "aucRevNaksPerRound", off: 5, size: 3, ty: "byte", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEERect", size: 8, align: 2, packed: false, declared_in: "AEERect.h",
        fields: &[FieldInfo { name: "x", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "y", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "dx", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "dy", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEERegItem", size: 16, align: 4, packed: false, declared_in: "AEERegistry.h",
        fields: &[FieldInfo { name: "clsType", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "clsHandler", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pszMime", off: 8, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "bActive", off: 12, size: 4, ty: "flg", bits: 1, bit_off: 0 }, FieldInfo { name: "bRestricted", off: 12, size: 4, ty: "flg", bits: 1, bit_off: 1 }],
    },
    StructInfo {
        name: "AEERemoteControl_CommandType", size: 12, align: 4, packed: false, declared_in: "AEEIRemoteControl_Target.h",
        fields: &[FieldInfo { name: "Type", off: 0, size: 8, ty: "uint64struct", bits: 0, bit_off: 0 }, FieldInfo { name: "Direction", off: 8, size: 4, ty: "AEERemoteControl_OpDirection", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEERemoteControl_VendorCommandType", size: 16, align: 4, packed: false, declared_in: "AEEIRemoteControl_Target.h",
        fields: &[FieldInfo { name: "TransportID", off: 0, size: 4, ty: "AEERemoteControl_Transport", bits: 0, bit_off: 0 }, FieldInfo { name: "VendorData", off: 4, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "VendorDataLen", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "VendorDataLenReq", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEERemoteControl___seq_unsigned_long", size: 12, align: 4, packed: false, declared_in: "AEEIRemoteControl_Target.h",
        fields: &[FieldInfo { name: "data", off: 0, size: 4, ty: "AEERemoteControl_Transport*", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLen", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLenReq", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEResBlob", size: 3, align: 1, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "bDataOffset", off: 0, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "bZero", off: 1, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "szMimeType", off: 2, size: 1, ty: "char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEERingerCat", size: 72, align: 4, packed: false, declared_in: "AEERinger.h",
        fields: &[FieldInfo { name: "id", off: 0, size: 4, ty: "AEERingerCatID", bits: 0, bit_off: 0 }, FieldInfo { name: "idRinger", off: 4, size: 4, ty: "AEERingerID", bits: 0, bit_off: 0 }, FieldInfo { name: "szName", off: 8, size: 64, ty: "AECHAR", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEERingerInfo", size: 140, align: 4, packed: false, declared_in: "AEERinger.h",
        fields: &[FieldInfo { name: "id", off: 0, size: 4, ty: "AEERingerID", bits: 0, bit_off: 0 }, FieldInfo { name: "format", off: 4, size: 4, ty: "AEESoundPlayerFile", bits: 0, bit_off: 0 }, FieldInfo { name: "bReadOnly", off: 8, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "szFile", off: 9, size: 64, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "szName", off: 74, size: 64, ty: "AECHAR", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESDBAckCBType", size: 8, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "pfn", off: 0, size: 4, ty: "PFNSDBACKEVENT", bits: 0, bit_off: 0 }, FieldInfo { name: "pUserData", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESDBAckInfo", size: 8, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "bOverFlow", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "nStatus", off: 4, size: 4, ty: "AEESDBAckStatus", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESIOConfig", size: 20, align: 4, packed: false, declared_in: "AEESio.h",
        fields: &[FieldInfo { name: "dwBaudRate", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nDataBits", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nStopBits", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "fFlowControl", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "fParity", off: 16, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESIOTriggers", size: 12, align: 4, packed: false, declared_in: "AEESio.h",
        fields: &[FieldInfo { name: "nReadIdleTimeout", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "cbReadTrigger", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "cbWriteTrigger", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESMSBCConfigEvtData", size: 16, align: 4, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "mode", off: 0, size: 4, ty: "AEESMSMode", bits: 0, bit_off: 0 }, FieldInfo { name: "sSrvID", off: 4, size: 12, ty: "AEESMSBCSrvID", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESMSBCSrvID", size: 12, align: 4, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "srvType", off: 0, size: 4, ty: "AEESMSBCType", bits: 0, bit_off: 0 }, FieldInfo { name: "dwLang", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "fromService", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "toService", off: 10, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESMSCSChangeEvtData", size: 12, align: 4, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "mt", off: 0, size: 4, ty: "AEESMSType", bits: 0, bit_off: 0 }, FieldInfo { name: "oldCS", off: 4, size: 4, ty: "AEESMSClientStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "newCS", off: 8, size: 4, ty: "AEESMSClientStatus", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESMSGWTime", size: 16, align: 2, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "wYear", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wMonth", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wDay", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wHour", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wMinute", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wSecond", off: 10, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wWeekDay", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nTimeZone", off: 14, size: 2, ty: "sint15", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESMSMsg", size: 728, align: 4, packed: false, declared_in: "AEETAPI.h",
        fields: &[FieldInfo { name: "teleservice", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "ts", off: 2, size: 6, ty: "AEESMSTimestamp", bits: 0, bit_off: 0 }, FieldInfo { name: "priority", off: 8, size: 4, ty: "AEESMSPriority", bits: 0, bit_off: 0 }, FieldInfo { name: "privacy", off: 12, size: 4, ty: "AEESMSPrivacy", bits: 0, bit_off: 0 }, FieldInfo { name: "nMessages", off: 16, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "bAlert", off: 20, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "dwLang", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nDigits", off: 28, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "encoding", off: 32, size: 4, ty: "AEESMSEncoding", bits: 0, bit_off: 0 }, FieldInfo { name: "is91", off: 36, size: 4, ty: "AEESMSIS91EP", bits: 0, bit_off: 0 }, FieldInfo { name: "nLen", off: 40, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "data", off: 41, size: 687, ty: "byte", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESMSStorageEvtData", size: 12, align: 4, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "st", off: 0, size: 4, ty: "AEESMSStorageType", bits: 0, bit_off: 0 }, FieldInfo { name: "nIndex", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "mt", off: 8, size: 4, ty: "AEESMSTag", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESMSStorageStatus", size: 16, align: 4, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "nSize", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nMaxSlots", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nFreeSlots", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nUsedTagSlots", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESMSTextMsg", size: 4, align: 2, packed: false, declared_in: "AEETAPI.h",
        fields: &[FieldInfo { name: "nChars", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "szText", off: 2, size: 1, ty: "char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESMSTimestamp", size: 6, align: 1, packed: false, declared_in: "AEETAPI.h",
        fields: &[FieldInfo { name: "year", off: 0, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "month", off: 1, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "day", off: 2, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "hour", off: 3, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "minute", off: 4, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "second", off: 5, size: 1, ty: "byte", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEScreenSaverInfo", size: 6, align: 2, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "wSecs", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wSecsMin", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wSecsMax", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESectorInfo", size: 24, align: 4, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "wSysID", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wNetID", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wBaseID", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wBaseClass", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wBestPN", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wPacketZoneID", off: 10, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wMobileCountryCode", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBaseLatitude", off: 16, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBaseLongitude", off: 20, size: 4, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESize", size: 8, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "cx", off: 0, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 4, size: 4, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockAddrInet", size: 8, align: 4, packed: false, declared_in: "AEENetAddrTypes.h",
        fields: &[FieldInfo { name: "wFamily", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "port", off: 2, size: 2, ty: "INPort", bits: 0, bit_off: 0 }, FieldInfo { name: "addr", off: 4, size: 4, ty: "INAddr", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockAddrInet6", size: 28, align: 4, packed: false, declared_in: "AEENetAddrTypes.h",
        fields: &[FieldInfo { name: "wFamily", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "port", off: 2, size: 2, ty: "INPort", bits: 0, bit_off: 0 }, FieldInfo { name: "dwFlowinfo", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "addr", off: 8, size: 16, ty: "INAddr6", bits: 0, bit_off: 0 }, FieldInfo { name: "dwScopeId", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockAddrLocal", size: 130, align: 2, packed: false, declared_in: "AEENetAddrTypes.h",
        fields: &[FieldInfo { name: "wFamily", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "szName", off: 2, size: 128, ty: "char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockAddrStorage", size: 130, align: 4, packed: false, declared_in: "AEENetAddrTypes.h",
        fields: &[FieldInfo { name: "wFamily", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "local", off: 0, size: 130, ty: "AEESockAddrLocal", bits: 0, bit_off: 0 }, FieldInfo { name: "inet", off: 0, size: 8, ty: "AEESockAddrInet", bits: 0, bit_off: 0 }, FieldInfo { name: "inet6", off: 0, size: 28, ty: "AEESockAddrInet6", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockCMsgHdr", size: 12, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "uLen", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nLevel", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nType", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockExtendedErr", size: 16, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "nError", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "ucOrigin", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "ucType", off: 5, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "ucCode", off: 6, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uInfo", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uData", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockMsgHdr", size: 28, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "pName", off: 0, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "wNameLen", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "pIov", off: 8, size: 4, ty: "SockIOBlock*", bits: 0, bit_off: 0 }, FieldInfo { name: "wIovLen", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "pControl", off: 16, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "wControlLen", off: 20, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "uFlags", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockOpt", size: 16, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "nLevel", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nOptName", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pOptVal", off: 8, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnOptSize", off: 12, size: 4, ty: "int*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockPortRecvMsg", size: 8, align: 4, packed: false, declared_in: "AEEISockPort.h",
        fields: &[FieldInfo { name: "pmh", off: 0, size: 4, ty: "AEESockMsgHdr*", bits: 0, bit_off: 0 }, FieldInfo { name: "uFlags", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESockPortSelectNetworkEx", size: 8, align: 4, packed: false, declared_in: "AEEISockPort.h",
        fields: &[FieldInfo { name: "nNetwork", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nFamily", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESoundCmdData", size: 2, align: 2, packed: false, declared_in: "AEEISound.h",
        fields: &[FieldInfo { name: "wVolume", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wPlayIndex", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESoundInfo", size: 20, align: 4, packed: false, declared_in: "AEEISound.h",
        fields: &[FieldInfo { name: "eDevice", off: 0, size: 4, ty: "AEESoundDevice", bits: 0, bit_off: 0 }, FieldInfo { name: "eMethod", off: 4, size: 4, ty: "AEESoundMethod", bits: 0, bit_off: 0 }, FieldInfo { name: "eAPath", off: 8, size: 4, ty: "AEESoundAPath", bits: 0, bit_off: 0 }, FieldInfo { name: "eEarMuteCtl", off: 12, size: 4, ty: "AEESoundMuteCtl", bits: 0, bit_off: 0 }, FieldInfo { name: "eMicMuteCtl", off: 16, size: 4, ty: "AEESoundMuteCtl", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESoundPlayerAudioSpec", size: 168, align: 4, packed: false, declared_in: "AEESoundPlayer.h",
        fields: &[FieldInfo { name: "fType", off: 0, size: 4, ty: "AEESoundPlayerFile", bits: 0, bit_off: 0 }, FieldInfo { name: "MIDISpec", off: 0, size: 4, ty: "AEESoundPlayerMIDISpec", bits: 0, bit_off: 0 }, FieldInfo { name: "MP3Spec", off: 0, size: 168, ty: "AEESoundPlayerMP3Spec", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESoundPlayerCmdData", size: 168, align: 4, packed: false, declared_in: "AEESoundPlayer.h",
        fields: &[FieldInfo { name: "dwElapsedTime", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTotalTime", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTempo", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwTune", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwPan", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wVolume", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "spSpec", off: 0, size: 168, ty: "AEESoundPlayerAudioSpec", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESoundPlayerInfo", size: 12, align: 4, packed: false, declared_in: "AEESoundPlayer.h",
        fields: &[FieldInfo { name: "eInput", off: 0, size: 4, ty: "AEESoundPlayerInput", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSize", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESoundPlayerMIDISpec", size: 4, align: 4, packed: false, declared_in: "AEESoundPlayer.h",
        fields: &[FieldInfo { name: "fType", off: 0, size: 4, ty: "AEESoundPlayerFile", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESoundPlayerMP3Spec", size: 168, align: 4, packed: false, declared_in: "AEESoundPlayer.h",
        fields: &[FieldInfo { name: "fType", off: 0, size: 4, ty: "AEESoundPlayerFile", bits: 0, bit_off: 0 }, FieldInfo { name: "version", off: 4, size: 4, ty: "AEESoundPlayerMP3Version", bits: 0, bit_off: 0 }, FieldInfo { name: "layer", off: 8, size: 4, ty: "AEESoundPlayerMP3Layer", bits: 0, bit_off: 0 }, FieldInfo { name: "crcFlag", off: 12, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bitrate", off: 16, size: 4, ty: "AEESoundPlayerMP3BitRate", bits: 0, bit_off: 0 }, FieldInfo { name: "sampleRate", off: 20, size: 4, ty: "AEESoundPlayerMP3SampleRate", bits: 0, bit_off: 0 }, FieldInfo { name: "channel", off: 24, size: 4, ty: "AEESoundPlayerMP3Channel", bits: 0, bit_off: 0 }, FieldInfo { name: "extension", off: 28, size: 4, ty: "AEESoundPlayerMP3Extension", bits: 0, bit_off: 0 }, FieldInfo { name: "copyrightFlag", off: 32, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "originalFlag", off: 33, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "emphasis", off: 36, size: 4, ty: "AEESoundPlayerMP3Emphasis", bits: 0, bit_off: 0 }, FieldInfo { name: "title", off: 40, size: 30, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "artist", off: 70, size: 30, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "album", off: 100, size: 30, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "year", off: 130, size: 4, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "comment", off: 134, size: 30, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "genre", off: 164, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESoundToneData", size: 8, align: 4, packed: false, declared_in: "AEEISound.h",
        fields: &[FieldInfo { name: "eTone", off: 0, size: 4, ty: "AEESoundTone", bits: 0, bit_off: 0 }, FieldInfo { name: "wDuration", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESpriteCmd", size: 16, align: 2, packed: false, declared_in: "AEESprite.h",
        fields: &[FieldInfo { name: "x", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "y", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "unTransform", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "unMatrixTransform", off: 6, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "unSpriteIndex", off: 7, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "unSpriteSize", off: 8, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "unComposite", off: 9, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "unLayer", off: 10, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "reserved", off: 11, size: 5, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESuspendInfo", size: 8, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "nErr", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "bCloseDialogs", off: 4, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEESysError", size: 12, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "cls", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nErr", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nType", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETAOCInfo", size: 12, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "ccm", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "acm", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "acm_max", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETCallEvent", size: 752, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "evCode", off: 0, size: 4, ty: "AEETEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "pModel", off: 4, size: 4, ty: "IModel*", bits: 0, bit_off: 0 }, FieldInfo { name: "call", off: 8, size: 744, ty: "AEETCallEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETCallEventData", size: 744, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "cd", off: 0, size: 4, ty: "AEECallDesc", bits: 0, bit_off: 0 }, FieldInfo { name: "number", off: 4, size: 108, ty: "AEETNumber", bits: 0, bit_off: 0 }, FieldInfo { name: "alpha", off: 112, size: 130, ty: "AECHAR", bits: 0, bit_off: 0 }, FieldInfo { name: "disp_type", off: 244, size: 4, ty: "AEETDispType", bits: 0, bit_off: 0 }, FieldInfo { name: "signal", off: 248, size: 16, ty: "AEETSignal", bits: 0, bit_off: 0 }, FieldInfo { name: "end_status", off: 264, size: 4, ty: "AEETEndStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "missed", off: 268, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "end_additional_info", off: 272, size: 4, ty: "AEETEndCause", bits: 0, bit_off: 0 }, FieldInfo { name: "ect_info", off: 276, size: 8, ty: "AEETECTInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "call_error", off: 284, size: 4, ty: "AEETCallError", bits: 0, bit_off: 0 }, FieldInfo { name: "callops_error", off: 288, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "call_info", off: 292, size: 452, ty: "AEETCallInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETCallInfo", size: 452, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "call_state", off: 0, size: 4, ty: "AEETCallState", bits: 0, bit_off: 0 }, FieldInfo { name: "call_type", off: 4, size: 4, ty: "AEETCallType", bits: 0, bit_off: 0 }, FieldInfo { name: "sys_mode", off: 8, size: 4, ty: "AEETSysMode", bits: 0, bit_off: 0 }, FieldInfo { name: "direction", off: 12, size: 4, ty: "AEETCallDirection", bits: 0, bit_off: 0 }, FieldInfo { name: "start_time", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "duration", off: 20, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "alpha", off: 24, size: 130, ty: "AECHAR", bits: 0, bit_off: 0 }, FieldInfo { name: "other_party_no", off: 154, size: 65, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "other_party_subaddr", off: 219, size: 22, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "redirect_party_number", off: 244, size: 108, ty: "AEETNumber", bits: 0, bit_off: 0 }, FieldInfo { name: "forward_cug_info", off: 352, size: 6, ty: "AEETFwdCugInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "cm_clir", off: 360, size: 4, ty: "AEETCLIR", bits: 0, bit_off: 0 }, FieldInfo { name: "cause_of_no_cli", off: 364, size: 4, ty: "AEETNoCLICause", bits: 0, bit_off: 0 }, FieldInfo { name: "srv_opt", off: 368, size: 4, ty: "AEETServiceOption", bits: 0, bit_off: 0 }, FieldInfo { name: "privacy_pref", off: 372, size: 4, ty: "AEETPrivacyPref", bits: 0, bit_off: 0 }, FieldInfo { name: "is_privacy", off: 376, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "alert_ans_bypass", off: 377, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "forwarded_from", off: 378, size: 65, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "call_progress", off: 444, size: 4, ty: "AEETCallProgress", bits: 0, bit_off: 0 }, FieldInfo { name: "line_id", off: 448, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETCalls", size: 8, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "dwCount", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "calls", off: 4, size: 4, ty: "AEECallDesc", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETCugIndex", size: 3, align: 1, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "present", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "msb", off: 1, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "lsb", off: 2, size: 1, ty: "byte", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETDetailedPLMNInfo", size: 28, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "plmn", off: 0, size: 3, ty: "AEETPLMNID", bits: 0, bit_off: 0 }, FieldInfo { name: "rat", off: 4, size: 4, ty: "AEETRadioAccessTech", bits: 0, bit_off: 0 }, FieldInfo { name: "plmn_service_capability", off: 8, size: 4, ty: "AEETSrvCapability", bits: 0, bit_off: 0 }, FieldInfo { name: "plmn_forbidden", off: 12, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "list_category", off: 16, size: 4, ty: "AEETDetailedPLMNCat", bits: 0, bit_off: 0 }, FieldInfo { name: "signal_quality", off: 20, size: 4, ty: "AEETSignalQuality", bits: 0, bit_off: 0 }, FieldInfo { name: "signal_strength", off: 24, size: 4, ty: "AEETSignalStrength", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETDetailedPLMNList", size: 1124, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "length", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "info", off: 4, size: 1120, ty: "AEETDetailedPLMNInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETECTInfo", size: 8, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "present", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "ect_state", off: 4, size: 4, ty: "AEETECTState", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETEventData", size: 1292, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "call", off: 0, size: 744, ty: "AEETCallEventData", bits: 0, bit_off: 0 }, FieldInfo { name: "ph", off: 0, size: 1292, ty: "AEETPhEventData", bits: 0, bit_off: 0 }, FieldInfo { name: "ss", off: 0, size: 688, ty: "AEETSSEventData", bits: 0, bit_off: 0 }, FieldInfo { name: "inb", off: 0, size: 520, ty: "AEETInbandEventData", bits: 0, bit_off: 0 }, FieldInfo { name: "supps", off: 0, size: 8, ty: "AEETSuppsEventData", bits: 0, bit_off: 0 }, FieldInfo { name: "user", off: 0, size: 588, ty: "AEETUserEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETFwdCugInfo", size: 6, align: 1, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "present", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "suppress_pref_cug", off: 1, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "suppress_outgoing_access", off: 2, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "cug_index", off: 3, size: 3, ty: "AEETCugIndex", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETIS95SysID", size: 4, align: 2, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "sid", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nid", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETInbandEvent", size: 528, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "evCode", off: 0, size: 4, ty: "AEETEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "pModel", off: 4, size: 4, ty: "IModel*", bits: 0, bit_off: 0 }, FieldInfo { name: "inb", off: 8, size: 520, ty: "AEETInbandEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETInbandEventData", size: 520, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "cd", off: 0, size: 4, ty: "AEECallDesc", bits: 0, bit_off: 0 }, FieldInfo { name: "on_length", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "off_length", off: 5, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "digits", off: 6, size: 514, ty: "AECHAR", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETLSAIdentity", size: 8, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "type", off: 0, size: 4, ty: "AEETLSAIdentityType", bits: 0, bit_off: 0 }, FieldInfo { name: "identity", off: 4, size: 3, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETLineInfo", size: 12, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "dwMaxSupportedLines", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSelectedLineId", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "bLineSwitchingEnabled", off: 8, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETMMInfo", size: 568, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "plmn_avail", off: 0, size: 4, ty: "unsigned", bits: 1, bit_off: 0 }, FieldInfo { name: "full_name_avail", off: 0, size: 4, ty: "unsigned", bits: 1, bit_off: 1 }, FieldInfo { name: "short_name_avail", off: 0, size: 4, ty: "unsigned", bits: 1, bit_off: 2 }, FieldInfo { name: "time_and_time_zone_avail", off: 0, size: 4, ty: "unsigned", bits: 1, bit_off: 3 }, FieldInfo { name: "time_zone_avail", off: 0, size: 4, ty: "unsigned", bits: 1, bit_off: 4 }, FieldInfo { name: "lsa_identity_avail", off: 0, size: 4, ty: "unsigned", bits: 1, bit_off: 5 }, FieldInfo { name: "daylight_saving_adj", off: 0, size: 4, ty: "unsigned", bits: 2, bit_off: 6 }, FieldInfo { name: "plmn", off: 4, size: 3, ty: "AEETPLMNID", bits: 0, bit_off: 0 }, FieldInfo { name: "full_name", off: 8, size: 264, ty: "AEETNWName", bits: 0, bit_off: 0 }, FieldInfo { name: "short_name", off: 272, size: 264, ty: "AEETNWName", bits: 0, bit_off: 0 }, FieldInfo { name: "time_and_time_zone", off: 536, size: 20, ty: "AEETTimeAndZone", bits: 0, bit_off: 0 }, FieldInfo { name: "time_zone", off: 556, size: 4, ty: "AEETTimeZone", bits: 0, bit_off: 0 }, FieldInfo { name: "lsa_identity", off: 560, size: 8, ty: "AEETLSAIdentity", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETNWName", size: 264, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "length", off: 0, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "coding_scheme", off: 4, size: 4, ty: "AEETNWCodingType", bits: 0, bit_off: 0 }, FieldInfo { name: "add_country_initials", off: 8, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "name", off: 9, size: 255, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETNotifyInfo", size: 1296, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "event", off: 0, size: 4, ty: "AEETEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "event_data", off: 4, size: 1292, ty: "AEETEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETNumber", size: 108, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "buf", off: 0, size: 65, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "number_type", off: 68, size: 4, ty: "AEETNumberType", bits: 0, bit_off: 0 }, FieldInfo { name: "number_plan", off: 72, size: 4, ty: "AEETNumberPlan", bits: 0, bit_off: 0 }, FieldInfo { name: "pi", off: 76, size: 4, ty: "AEETPI", bits: 0, bit_off: 0 }, FieldInfo { name: "si", off: 80, size: 4, ty: "AEETSI", bits: 0, bit_off: 0 }, FieldInfo { name: "subaddr", off: 84, size: 24, ty: "AEETSubAddress", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETPLMNID", size: 3, align: 1, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "identity", off: 0, size: 3, ty: "byte", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETPhInfo", size: 1292, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "is_in_use", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "oprt_mode", off: 4, size: 4, ty: "AEETOprtMode", bits: 0, bit_off: 0 }, FieldInfo { name: "test_control_type", off: 8, size: 4, ty: "AEETTestControlType", bits: 0, bit_off: 0 }, FieldInfo { name: "cdma_lock_mode", off: 12, size: 4, ty: "AEETCDMALockMode", bits: 0, bit_off: 0 }, FieldInfo { name: "answer_voice", off: 16, size: 4, ty: "AEETAnsVoicePref", bits: 0, bit_off: 0 }, FieldInfo { name: "mode_pref", off: 20, size: 4, ty: "AEETModePref", bits: 0, bit_off: 0 }, FieldInfo { name: "pref_term", off: 24, size: 4, ty: "AEETPrefTerm", bits: 0, bit_off: 0 }, FieldInfo { name: "acq_order_pref", off: 28, size: 4, ty: "AEETAcqOrderPref", bits: 0, bit_off: 0 }, FieldInfo { name: "srv_domain_pref", off: 32, size: 4, ty: "AEETServDomPref", bits: 0, bit_off: 0 }, FieldInfo { name: "band_pref", off: 36, size: 4, ty: "AEETBandPref", bits: 0, bit_off: 0 }, FieldInfo { name: "roam_pref", off: 40, size: 4, ty: "AEETRoamPref", bits: 0, bit_off: 0 }, FieldInfo { name: "nam_sel", off: 44, size: 4, ty: "AEETNAM", bits: 0, bit_off: 0 }, FieldInfo { name: "curr_nam", off: 48, size: 4, ty: "AEETNAM", bits: 0, bit_off: 0 }, FieldInfo { name: "hybr_pref", off: 52, size: 4, ty: "AEETHybrPref", bits: 0, bit_off: 0 }, FieldInfo { name: "network_sel_mode_pref", off: 56, size: 4, ty: "AEETNwSelModePref", bits: 0, bit_off: 0 }, FieldInfo { name: "plmn", off: 60, size: 3, ty: "AEETPLMNID", bits: 0, bit_off: 0 }, FieldInfo { name: "user_pref_networks", off: 64, size: 62, ty: "AEETUserPLMNList", bits: 0, bit_off: 0 }, FieldInfo { name: "available_networks", off: 128, size: 1124, ty: "AEETDetailedPLMNList", bits: 0, bit_off: 0 }, FieldInfo { name: "is_cdma_subscription_available", off: 1252, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "is_gw_subscription_available", off: 1253, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "rtre_config", off: 1256, size: 4, ty: "AEETRTREConfig", bits: 0, bit_off: 0 }, FieldInfo { name: "rtre_control", off: 1260, size: 4, ty: "AEETRTREControl", bits: 0, bit_off: 0 }, FieldInfo { name: "aoc_info", off: 1264, size: 12, ty: "AEETAOCInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "ddtm_pref", off: 1276, size: 4, ty: "AEETDDTMPref", bits: 0, bit_off: 0 }, FieldInfo { name: "line_info", off: 1280, size: 12, ty: "AEETLineInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETPhoneEvent", size: 1300, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "evCode", off: 0, size: 4, ty: "AEETEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "pModel", off: 4, size: 4, ty: "IModel*", bits: 0, bit_off: 0 }, FieldInfo { name: "ph", off: 8, size: 1292, ty: "AEETPhEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETPrefNetworks", size: 62, align: 2, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "pref_networks", off: 0, size: 62, ty: "AEETUserPLMNList", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSSEvent", size: 696, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "evCode", off: 0, size: 4, ty: "AEETEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "pModel", off: 4, size: 4, ty: "IModel*", bits: 0, bit_off: 0 }, FieldInfo { name: "ss", off: 8, size: 688, ty: "AEETSSEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSSEventData", size: 688, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "changed_fields", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_info", off: 4, size: 684, ty: "AEETSSInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSSInfo", size: 684, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "srv_status", off: 0, size: 4, ty: "AEETSrvStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "srv_domain", off: 4, size: 4, ty: "AEETSrvDomain", bits: 0, bit_off: 0 }, FieldInfo { name: "srv_capability", off: 8, size: 4, ty: "AEETSrvCapability", bits: 0, bit_off: 0 }, FieldInfo { name: "sys_mode", off: 12, size: 4, ty: "AEETSysMode", bits: 0, bit_off: 0 }, FieldInfo { name: "roam_status", off: 16, size: 4, ty: "AEETRoamStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "sys_id", off: 20, size: 20, ty: "AEETSysID", bits: 0, bit_off: 0 }, FieldInfo { name: "rssi", off: 40, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "hdr_hybrid", off: 42, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "tssinfo_version", off: 43, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "hdr_srv_status", off: 44, size: 4, ty: "AEETSrvStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "hdr_roam_status", off: 48, size: 4, ty: "AEETRoamStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "hdr_rssi", off: 52, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wlan_rssi", off: 54, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "sim_state", off: 56, size: 4, ty: "AEETSIMState", bits: 0, bit_off: 0 }, FieldInfo { name: "plmn_forbidden", off: 60, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "mm_information", off: 64, size: 568, ty: "AEETMMInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "band_class", off: 632, size: 4, ty: "AEETSysBandClass", bits: 0, bit_off: 0 }, FieldInfo { name: "block_or_system", off: 636, size: 4, ty: "AEETSysBlocks", bits: 0, bit_off: 0 }, FieldInfo { name: "is_registered", off: 640, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "uz_id", off: 644, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "uz_name", off: 648, size: 34, ty: "AEETUZName", bits: 0, bit_off: 0 }, FieldInfo { name: "ccs_supported", off: 682, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "p_rev_in_use", off: 683, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSignal", size: 16, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "is_signal_info_avail", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "signal_type", off: 4, size: 4, ty: "AEETSignalType", bits: 0, bit_off: 0 }, FieldInfo { name: "signal", off: 8, size: 4, ty: "AEETSignalValue", bits: 0, bit_off: 0 }, FieldInfo { name: "signal_pitch", off: 12, size: 4, ty: "AEETSignalPitch", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSignalValue", size: 4, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "gw_tone", off: 0, size: 4, ty: "AEETGWSignalTone", bits: 0, bit_off: 0 }, FieldInfo { name: "cdma_tone", off: 0, size: 4, ty: "AEETCDMASignalTone", bits: 0, bit_off: 0 }, FieldInfo { name: "cdma_isdn", off: 0, size: 4, ty: "AEETCDMASignalISDN", bits: 0, bit_off: 0 }, FieldInfo { name: "cdma_is54b", off: 0, size: 4, ty: "AEETCDMASignalIS54B", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSubAddress", size: 24, align: 1, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "extension_bit", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "odd_even_ind", off: 1, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "addr", off: 2, size: 22, ty: "char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsBasicServiceGroupListInfo", size: 56, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "length", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "bsg_list", off: 4, size: 52, ty: "AEETSuppsBasicServiceType", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsCCResponse", size: 524, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "cc_op", off: 0, size: 4, ty: "AEETSuppsCCOp", bits: 0, bit_off: 0 }, FieldInfo { name: "alpha_count", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "alpha", off: 8, size: 512, ty: "AECHAR", bits: 0, bit_off: 0 }, FieldInfo { name: "alpha_valid", off: 520, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsCallBarringFeature", size: 8, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "basic_service", off: 0, size: 4, ty: "AEETSuppsBasicServiceType", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_status", off: 4, size: 4, ty: "AEETSuppsOpStatus", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsCallBarringInfo", size: 112, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "ss_code", off: 0, size: 4, ty: "AEETSuppsCodeType", bits: 0, bit_off: 0 }, FieldInfo { name: "length", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "call_barring_feature_list", off: 8, size: 104, ty: "AEETSuppsCallBarringFeature", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsChangePassword", size: 27, align: 1, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "szCurPasswd", off: 0, size: 9, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "szNewPasswd", off: 9, size: 9, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "szNewPasswdRepeated", off: 18, size: 9, ty: "char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsCliRestrictInfo", size: 8, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "ss_status", off: 0, size: 4, ty: "AEETSuppsOpStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "cli_restriction_option", off: 4, size: 4, ty: "AEETSuppsCliRestrictOptionType", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsData", size: 8, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "length", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "data", off: 4, size: 4, ty: "char*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsDataEx", size: 12, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "length", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "data", off: 4, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "data_encoding", off: 8, size: 2, ty: "AEETSuppsEncodingType", bits: 0, bit_off: 0 }, FieldInfo { name: "broadcast_encoding", off: 10, size: 2, ty: "AEETSuppsEncodingType", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsDataInfo", size: 76, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "length", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_code", off: 4, size: 4, ty: "AEETSuppsCodeType", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_status", off: 8, size: 4, ty: "AEETSuppsOpStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_subscription_option", off: 12, size: 8, ty: "AEETSuppsSubscriptionOption", bits: 0, bit_off: 0 }, FieldInfo { name: "basic_service_group_list", off: 20, size: 56, ty: "AEETSuppsBasicServiceGroupListInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsEvent", size: 16, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "evCode", off: 0, size: 4, ty: "AEETEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "pModel", off: 4, size: 4, ty: "IModel*", bits: 0, bit_off: 0 }, FieldInfo { name: "supps", off: 8, size: 8, ty: "AEETSuppsEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsEventData", size: 8, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "transaction_id", off: 0, size: 4, ty: "AEETSuppsTransId", bits: 0, bit_off: 0 }, FieldInfo { name: "ind_type", off: 4, size: 4, ty: "AEETSuppsNetworkIndType", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsForwardingInfo", size: 580, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "ss_code", off: 0, size: 4, ty: "AEETSuppsCodeType", bits: 0, bit_off: 0 }, FieldInfo { name: "feature_list", off: 4, size: 576, ty: "AEETSuppsFwdFeatureList", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsForwardingOption", size: 8, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "fwd_reason", off: 0, size: 4, ty: "AEETSuppsFwdReasonType", bits: 0, bit_off: 0 }, FieldInfo { name: "notification_to_fp", off: 4, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "notification_to_cp", off: 5, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsFwdFeature", size: 44, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "basic_service", off: 0, size: 4, ty: "AEETSuppsBasicServiceType", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_status", off: 4, size: 4, ty: "AEETSuppsOpStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "fwd_to_number", off: 8, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }, FieldInfo { name: "long_fwd_to_number", off: 16, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }, FieldInfo { name: "fwd_option", off: 24, size: 8, ty: "AEETSuppsForwardingOption", bits: 0, bit_off: 0 }, FieldInfo { name: "timer_value", off: 32, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "fwd_to_subaddress", off: 36, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsFwdFeatureList", size: 576, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "length", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "fwd_feature", off: 4, size: 572, ty: "AEETSuppsFwdFeature", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsMessage", size: 44, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "type", off: 0, size: 4, ty: "AEETSuppsMessageType", bits: 0, bit_off: 0 }, FieldInfo { name: "data", off: 4, size: 40, ty: "AEETSuppsMessage@1", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsMessage@1", size: 40, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "request", off: 0, size: 40, ty: "AEETSuppsRequest", bits: 0, bit_off: 0 }, FieldInfo { name: "response", off: 0, size: 12, ty: "AEETSuppsResponse", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsMessageEvent", size: 20, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "evCode", off: 0, size: 4, ty: "AEETEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "pModel", off: 4, size: 4, ty: "IModel*", bits: 0, bit_off: 0 }, FieldInfo { name: "supps_started", off: 8, size: 12, ty: "AEETSuppsMessageEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsMessageEventData", size: 12, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "pMsg", off: 0, size: 4, ty: "AEETSuppsMessage*", bits: 0, bit_off: 0 }, FieldInfo { name: "transaction_id", off: 4, size: 4, ty: "AEETSuppsTransId", bits: 0, bit_off: 0 }, FieldInfo { name: "pCCResponse", off: 8, size: 4, ty: "AEETSuppsCCResponse*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsNetworkInd", size: 12, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "ind_type", off: 0, size: 4, ty: "AEETSuppsNetworkIndType", bits: 0, bit_off: 0 }, FieldInfo { name: "ind_data", off: 4, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsOpStatus", size: 4, align: 1, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "ss_operative", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_provisioned", off: 1, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_registered", off: 2, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_active", off: 3, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsRegistration", size: 12, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "data", off: 0, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }, FieldInfo { name: "nr_timer", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsRegistrationEx", size: 20, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "data", off: 0, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }, FieldInfo { name: "nr_timer", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "TON", off: 12, size: 4, ty: "AEETNumberType", bits: 0, bit_off: 0 }, FieldInfo { name: "NPI", off: 16, size: 4, ty: "AEETNumberPlan", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsRequest", size: 40, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "operation", off: 0, size: 4, ty: "AEETSuppsRequestType", bits: 0, bit_off: 0 }, FieldInfo { name: "code", off: 4, size: 4, ty: "AEETSuppsCodeType", bits: 0, bit_off: 0 }, FieldInfo { name: "basic_service", off: 8, size: 4, ty: "AEETSuppsBasicServiceType", bits: 0, bit_off: 0 }, FieldInfo { name: "req", off: 12, size: 27, ty: "AEETSuppsRequest@3", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsRequest@3", size: 27, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "reg", off: 0, size: 12, ty: "AEETSuppsRegistration", bits: 0, bit_off: 0 }, FieldInfo { name: "passwd", off: 0, size: 27, ty: "AEETSuppsChangePassword", bits: 0, bit_off: 0 }, FieldInfo { name: "ussd", off: 0, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }, FieldInfo { name: "regEx", off: 0, size: 20, ty: "AEETSuppsRegistrationEx", bits: 0, bit_off: 0 }, FieldInfo { name: "ussdEx", off: 0, size: 12, ty: "AEETSuppsDataEx", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsResponse", size: 12, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "response_type", off: 0, size: 4, ty: "AEETSuppsResType", bits: 0, bit_off: 0 }, FieldInfo { name: "response_data", off: 4, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsResult", size: 588, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "result_type", off: 0, size: 4, ty: "AEETSuppsResultType", bits: 0, bit_off: 0 }, FieldInfo { name: "success", off: 4, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "data", off: 8, size: 580, ty: "AEETSuppsResult@2", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsResult@2", size: 580, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "fwd_info", off: 0, size: 580, ty: "AEETSuppsForwardingInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "fwd_feature_list", off: 0, size: 576, ty: "AEETSuppsFwdFeatureList", bits: 0, bit_off: 0 }, FieldInfo { name: "call_barring_info", off: 0, size: 112, ty: "AEETSuppsCallBarringInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "cli_restriction", off: 0, size: 8, ty: "AEETSuppsCliRestrictInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "cug_info", off: 0, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }, FieldInfo { name: "bsg_list_info", off: 0, size: 56, ty: "AEETSuppsBasicServiceGroupListInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "uss_data_info", off: 0, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }, FieldInfo { name: "ussd_data_info", off: 0, size: 8, ty: "AEETSuppsData", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_new_pwd", off: 0, size: 9, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_data", off: 0, size: 76, ty: "AEETSuppsDataInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_error", off: 0, size: 4, ty: "AEETSuppsErrorType", bits: 0, bit_off: 0 }, FieldInfo { name: "ss_status", off: 0, size: 4, ty: "AEETSuppsOpStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "cc_cause", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "imei", off: 0, size: 16, ty: "char", bits: 0, bit_off: 0 }, FieldInfo { name: "pin_status", off: 0, size: 4, ty: "AEETSuppsPinStatus", bits: 0, bit_off: 0 }, FieldInfo { name: "cc_info", off: 0, size: 524, ty: "AEETSuppsCCResponse", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsResultEvent", size: 16, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "evCode", off: 0, size: 4, ty: "AEETEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "pModel", off: 4, size: 4, ty: "IModel*", bits: 0, bit_off: 0 }, FieldInfo { name: "supps_ended", off: 8, size: 8, ty: "AEETSuppsResultEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsResultEventData", size: 8, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "pRes", off: 0, size: 4, ty: "AEETSuppsResult*", bits: 0, bit_off: 0 }, FieldInfo { name: "transaction_id", off: 4, size: 4, ty: "AEETSuppsTransId", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsStatus", size: 592, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "status", off: 0, size: 4, ty: "AEETSuppsStatusType", bits: 0, bit_off: 0 }, FieldInfo { name: "data", off: 4, size: 588, ty: "AEETSuppsStatus@1", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsStatus@1", size: 588, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "NetworkInd", off: 0, size: 12, ty: "AEETSuppsNetworkInd", bits: 0, bit_off: 0 }, FieldInfo { name: "Result", off: 0, size: 588, ty: "AEETSuppsResult", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsSubscriptionOption", size: 8, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "option_type", off: 0, size: 4, ty: "AEETSuppsCliOptionType", bits: 0, bit_off: 0 }, FieldInfo { name: "option", off: 4, size: 4, ty: "AEETSuppsSubscriptionOption@1", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSuppsSubscriptionOption@1", size: 4, align: 4, packed: false, declared_in: "AEETSuppsDef.h",
        fields: &[FieldInfo { name: "cli_restrict", off: 0, size: 4, ty: "AEETSuppsCliRestrictOptionType", bits: 0, bit_off: 0 }, FieldInfo { name: "override_category", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSysID", size: 20, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "id_type", off: 0, size: 4, ty: "AEETSysIDType", bits: 0, bit_off: 0 }, FieldInfo { name: "id", off: 4, size: 16, ty: "AEETSysIDData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSysIDData", size: 16, align: 2, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "is95", off: 0, size: 4, ty: "AEETIS95SysID", bits: 0, bit_off: 0 }, FieldInfo { name: "is856", off: 0, size: 16, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "plmn", off: 0, size: 3, ty: "AEETPLMNID", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETSystemPreference", size: 40, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "mode_pref", off: 0, size: 4, ty: "AEETModePref", bits: 0, bit_off: 0 }, FieldInfo { name: "pref_term", off: 4, size: 4, ty: "AEETPrefTerm", bits: 0, bit_off: 0 }, FieldInfo { name: "pref_duration", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "acq_order_pref", off: 12, size: 4, ty: "AEETAcqOrderPref", bits: 0, bit_off: 0 }, FieldInfo { name: "band_pref", off: 16, size: 4, ty: "AEETBandPref", bits: 0, bit_off: 0 }, FieldInfo { name: "roam_pref", off: 20, size: 4, ty: "AEETRoamPref", bits: 0, bit_off: 0 }, FieldInfo { name: "hybr_pref", off: 24, size: 4, ty: "AEETHybrPref", bits: 0, bit_off: 0 }, FieldInfo { name: "srv_domain_pref", off: 28, size: 4, ty: "AEETServDomPref", bits: 0, bit_off: 0 }, FieldInfo { name: "nw_sel_mode_pref", off: 32, size: 4, ty: "AEETNwSelModePref", bits: 0, bit_off: 0 }, FieldInfo { name: "plmn", off: 36, size: 3, ty: "AEETPLMNID", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETTimeAndZone", size: 20, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "time", off: 0, size: 14, ty: "AEEDateTime", bits: 0, bit_off: 0 }, FieldInfo { name: "time_zone", off: 16, size: 4, ty: "AEETTimeZone", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETUZName", size: 34, align: 1, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "uz_name_encoding", off: 0, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "uz_name_n_octets", off: 1, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "uz_name", off: 2, size: 32, ty: "byte", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETUserDataInfo", size: 132, align: 2, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "type", off: 0, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "length", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "user_data", off: 4, size: 128, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETUserEvent", size: 596, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "evCode", off: 0, size: 4, ty: "AEETEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "pModel", off: 4, size: 4, ty: "IModel*", bits: 0, bit_off: 0 }, FieldInfo { name: "user", off: 8, size: 588, ty: "AEETUserEventData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETUserEventData", size: 588, align: 4, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "cd", off: 0, size: 4, ty: "AEECallDesc", bits: 0, bit_off: 0 }, FieldInfo { name: "call_info", off: 4, size: 452, ty: "AEETCallInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "user_data", off: 456, size: 132, ty: "AEETUserDataInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETUserPLMNInfo", size: 6, align: 2, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "plmn", off: 0, size: 3, ty: "AEETPLMNID", bits: 0, bit_off: 0 }, FieldInfo { name: "access_tech", off: 4, size: 2, ty: "AEETAccessTech", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETUserPLMNList", size: 62, align: 2, packed: false, declared_in: "AEETelDef.h",
        fields: &[FieldInfo { name: "length", off: 0, size: 1, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "info", off: 2, size: 60, ty: "AEETUserPLMNInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEText", size: 8, align: 4, packed: false, declared_in: "AEEDisp.h",
        fields: &[FieldInfo { name: "pszText", off: 0, size: 4, ty: "AECHAR*", bits: 0, bit_off: 0 }, FieldInfo { name: "pwLength", off: 4, size: 4, ty: "word*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETextInputModeInfo", size: 36, align: 4, packed: false, declared_in: "AEEText.h",
        fields: &[FieldInfo { name: "tmMode", off: 0, size: 4, ty: "AEETextInputMode", bits: 0, bit_off: 0 }, FieldInfo { name: "modeString", off: 4, size: 30, ty: "AECHAR", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETileMap", size: 40, align: 4, packed: false, declared_in: "AEESprite.h",
        fields: &[FieldInfo { name: "pMapArray", off: 0, size: 4, ty: "uint16*", bits: 0, bit_off: 0 }, FieldInfo { name: "unFlags", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "reserved", off: 8, size: 16, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "x", off: 24, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "y", off: 28, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "w", off: 32, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "h", off: 34, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "unTileSize", off: 36, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "reserved2", off: 37, size: 3, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETls", size: 8, align: 4, packed: false, declared_in: "AEETU_TLS.h",
        fields: &[FieldInfo { name: "piThread", off: 0, size: 4, ty: "IThread*", bits: 0, bit_off: 0 }, FieldInfo { name: "ptlssList", off: 4, size: 4, ty: "AEETlsSlot*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETransformMatrix", size: 8, align: 2, packed: false, declared_in: "AEEITransform.h",
        fields: &[FieldInfo { name: "A", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "B", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "C", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "D", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEETriangle", size: 12, align: 2, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "x0", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "y0", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "x1", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "y1", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "x2", off: 8, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "y2", off: 10, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEUDPUrgent", size: 4, align: 2, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "bUrgentSupported", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "nUrgentLimit", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEUMTSBearerTechInfo", size: 4, align: 4, packed: false, declared_in: "AEENetTypes.h",
        fields: &[FieldInfo { name: "uUMTSBearerTech", off: 0, size: 4, ty: "AEEUTMSBearerTech", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEUMTSInfo", size: 1, align: 1, packed: false, declared_in: "AEENet.h",
        fields: &[FieldInfo { name: "bValid", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEVAlarm", size: 20, align: 4, packed: false, declared_in: "AEEIvProperty.h",
        fields: &[FieldInfo { name: "nRpt_Cnt", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pszTrigger", off: 4, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "pszDuration", off: 8, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "pszStr1", off: 12, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "pszStr2", off: 16, size: 4, ty: "char*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEVData", size: 12, align: 4, packed: false, declared_in: "AEEIvProperty.h",
        fields: &[FieldInfo { name: "nValueType", off: 0, size: 4, ty: "AEEVValueType", bits: 0, bit_off: 0 }, FieldInfo { name: "dwDataSize", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 8, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEVDateTimePeriod", size: 32, align: 4, packed: false, declared_in: "AEEIvProperty.h",
        fields: &[FieldInfo { name: "dwValueType", off: 0, size: 4, ty: "AEEVValueType", bits: 0, bit_off: 0 }, FieldInfo { name: "bIsDuration", off: 4, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bIsPriorDuration", off: 5, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bIsUTC", off: 6, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "dwValue1", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwValue2", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "sDuration", off: 16, size: 14, ty: "AEEDateTime", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEVGeo", size: 16, align: 4, packed: false, declared_in: "AEEIvProperty.h",
        fields: &[FieldInfo { name: "nLongitude", off: 0, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }, FieldInfo { name: "nLatitude", off: 8, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEVRRule", size: 92, align: 4, packed: false, declared_in: "AEEIvProperty.h",
        fields: &[FieldInfo { name: "nFrequency", off: 0, size: 4, ty: "AEEVFrequency", bits: 0, bit_off: 0 }, FieldInfo { name: "dwCount", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwInterval", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "endDate", off: 12, size: 8, ty: "AEEVUTCDate", bits: 0, bit_off: 0 }, FieldInfo { name: "dwWeekDay", off: 20, size: 4, ty: "AEEVWeekDay", bits: 0, bit_off: 0 }, FieldInfo { name: "dwMonthInYear", off: 24, size: 4, ty: "AEEVMonth", bits: 0, bit_off: 0 }, FieldInfo { name: "dwWKST", off: 28, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSize", off: 32, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pnBySecList", off: 36, size: 4, ty: "uint32*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnByMinList", off: 40, size: 4, ty: "uint32*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnByHrList", off: 44, size: 4, ty: "uint32*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnWeekDayList", off: 48, size: 28, ty: "int32*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnDayInMonth", off: 76, size: 4, ty: "int32*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnDayInYear", off: 80, size: 4, ty: "int32*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnWeekNoList", off: 84, size: 4, ty: "int32*", bits: 0, bit_off: 0 }, FieldInfo { name: "pnBySpList", off: 88, size: 4, ty: "int32*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEVUTCDate", size: 8, align: 4, packed: false, declared_in: "AEEIvProperty.h",
        fields: &[FieldInfo { name: "bIsUTC", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "dwJulianSeconds", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEVersionDesc", size: 16, align: 4, packed: false, declared_in: "AEEIVersion.h",
        fields: &[FieldInfo { name: "nMajorVer", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nMinorVer", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nSubVer", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nBuildNum", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEVoicePrompt", size: 8, align: 4, packed: false, declared_in: "AEESoundPlayer.h",
        fields: &[FieldInfo { name: "pbData", off: 0, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "wNumFrames", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEEWIFIStatistics", size: 12, align: 4, packed: false, declared_in: "AEEWIFI.h",
        fields: &[FieldInfo { name: "dwSize", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "tx_bytes", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "rx_bytes", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_HASH_CTX", size: 224, align: 1, packed: false, declared_in: "AEESecurity.h",
        fields: &[FieldInfo { name: "ctx", off: 0, size: 224, ty: "AEE_HASH_CTX@0", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_HASH_CTX@0", size: 224, align: 1, packed: false, declared_in: "AEESecurity.h",
        fields: &[FieldInfo { name: "md5_ctx", off: 0, size: 88, ty: "AEE_MD5_CTX", bits: 0, bit_off: 0 }, FieldInfo { name: "md2_ctx", off: 0, size: 148, ty: "AEE_MD2_CTX", bits: 0, bit_off: 0 }, FieldInfo { name: "sha1_ctx", off: 0, size: 156, ty: "AEE_SHA1_CTX", bits: 0, bit_off: 0 }, FieldInfo { name: "sha256_ctx", off: 0, size: 104, ty: "AEE_SHA256_CTX", bits: 0, bit_off: 0 }, FieldInfo { name: "sha384_ctx", off: 0, size: 208, ty: "AEE_SHA384_CTX", bits: 0, bit_off: 0 }, FieldInfo { name: "sha512_ctx", off: 0, size: 208, ty: "AEE_SHA512_CTX", bits: 0, bit_off: 0 }, FieldInfo { name: "md5_hmac_ctx", off: 0, size: 156, ty: "AEE_MD5_HMAC_CTX", bits: 0, bit_off: 0 }, FieldInfo { name: "md2_hmac_ctx", off: 0, size: 216, ty: "AEE_MD2_HMAC_CTX", bits: 0, bit_off: 0 }, FieldInfo { name: "sha1_hmac_ctx", off: 0, size: 224, ty: "AEE_SHA1_HMAC_CTX", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_HMAC_MD2_CTX", size: 216, align: 1, packed: false, declared_in: "AEEHMACMD2Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 216, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_HMAC_MD5_CTX", size: 156, align: 1, packed: false, declared_in: "AEEHMACMD5Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 156, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_HMAC_SHA1_CTX", size: 224, align: 1, packed: false, declared_in: "AEEHMACSHA1Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 224, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_HMAC_SHA256_CTX", size: 176, align: 1, packed: false, declared_in: "AEEHMACSHA256Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 176, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_HMAC_SHA384_CTX", size: 344, align: 1, packed: false, declared_in: "AEEHMACSHA384Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 344, ty: "unsigned char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_HMAC_SHA512_CTX", size: 344, align: 1, packed: false, declared_in: "AEEHMACSHA512Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 344, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_MD2_CTX", size: 148, align: 1, packed: false, declared_in: "AEEMD2Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 148, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_MD5_CTX", size: 88, align: 1, packed: false, declared_in: "AEEMD5Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 88, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_SHA1_CTX", size: 156, align: 1, packed: false, declared_in: "AEESHA1Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 156, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_SHA256_CTX", size: 104, align: 1, packed: false, declared_in: "AEESHA256Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 104, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_SHA384_CTX", size: 208, align: 1, packed: false, declared_in: "AEESHA384Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 208, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AEE_SHA512_CTX", size: 208, align: 1, packed: false, declared_in: "AEESHA512Ctx.bid",
        fields: &[FieldInfo { name: "buf", off: 0, size: 208, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ASN1OIDMap", size: 12, align: 4, packed: false, declared_in: "AEEASN1OIDs.h",
        fields: &[FieldInfo { name: "uCount", off: 0, size: 4, ty: "unsigned int", bits: 0, bit_off: 0 }, FieldInfo { name: "pcPrefixes", off: 4, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "puASNOIDS", off: 8, size: 4, ty: "uint32*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ATITC_HEADER", size: 20, align: 4, packed: false, declared_in: "glESext.h",
        fields: &[FieldInfo { name: "signature", off: 0, size: 4, ty: "unsigned int", bits: 0, bit_off: 0 }, FieldInfo { name: "width", off: 4, size: 4, ty: "unsigned int", bits: 0, bit_off: 0 }, FieldInfo { name: "height", off: 8, size: 4, ty: "unsigned int", bits: 0, bit_off: 0 }, FieldInfo { name: "flags", off: 12, size: 4, ty: "unsigned int", bits: 0, bit_off: 0 }, FieldInfo { name: "dataOffset", off: 16, size: 4, ty: "unsigned int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "AppStartInfo", size: 12, align: 4, packed: false, declared_in: "AEEIAppletCtl.h",
        fields: &[FieldInfo { name: "wFlags", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "pszStart", off: 4, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwReason", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BDAddress", size: 6, align: 1, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "uAddr", off: 0, size: 6, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTA2DPEvent", size: 16, align: 4, packed: false, declared_in: "AEEIBTA2DPNotifier.h",
        fields: &[FieldInfo { name: "event", off: 0, size: 4, ty: "BTA2DPEventType", bits: 0, bit_off: 0 }, FieldInfo { name: "result", off: 4, size: 4, ty: "BTA2DPResult", bits: 0, bit_off: 0 }, FieldInfo { name: "remoteAddr", off: 8, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "bLocalInit", off: 14, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTAGConnection", size: 20, align: 4, packed: false, declared_in: "AEEIBTAudioGateway.h",
        fields: &[FieldInfo { name: "dConnState", off: 0, size: 4, ty: "BTConnectionState", bits: 0, bit_off: 0 }, FieldInfo { name: "dAudioConnState", off: 4, size: 4, ty: "BTConnectionState", bits: 0, bit_off: 0 }, FieldInfo { name: "remoteAddr", off: 8, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "dServiceUUID", off: 14, size: 2, ty: "BTServiceClassUUID", bits: 0, bit_off: 0 }, FieldInfo { name: "uServerChannel", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTAGStatus", size: 16, align: 4, packed: false, declared_in: "AEEIBTAudioGateway.h",
        fields: &[FieldInfo { name: "dState", off: 0, size: 4, ty: "BTAGState", bits: 0, bit_off: 0 }, FieldInfo { name: "dPendingOperation", off: 4, size: 4, ty: "BTAGOperationType", bits: 0, bit_off: 0 }, FieldInfo { name: "nNumConnections", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nMaxNumConnSupported", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTClassOfDevice", size: 6, align: 2, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "COS", off: 0, size: 2, ty: "BTClassOfService", bits: 0, bit_off: 0 }, FieldInfo { name: "DevCls", off: 2, size: 2, ty: "BTDeviceClass", bits: 0, bit_off: 0 }, FieldInfo { name: "SubDevCls", off: 4, size: 2, ty: "BTSubDeviceClass", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTConnEvData", size: 8, align: 2, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "bdAddr", off: 0, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "uuid", off: 6, size: 2, ty: "BTServiceClassUUID", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTConnStatus", size: 12, align: 4, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "ConnMode", off: 0, size: 4, ty: "BTConnectionMode", bits: 0, bit_off: 0 }, FieldInfo { name: "Security", off: 4, size: 4, ty: "BTSecurityLevel", bits: 0, bit_off: 0 }, FieldInfo { name: "bLocalIsMaster", off: 8, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTConnectionStatus", size: 52, align: 4, packed: false, declared_in: "AEEIBTConnection.h",
        fields: &[FieldInfo { name: "about", off: 0, size: 36, ty: "BTDeviceInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "dConnMode", off: 36, size: 4, ty: "BTConnectionMode", bits: 0, bit_off: 0 }, FieldInfo { name: "dSecurity", off: 40, size: 4, ty: "BTSecurityLevel", bits: 0, bit_off: 0 }, FieldInfo { name: "bLocalIsMaster", off: 44, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bRoleSwitchAllowed", off: 45, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bSCOPresent", off: 46, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "RSSI", off: 47, size: 1, ty: "int8", bits: 0, bit_off: 0 }, FieldInfo { name: "dPendingOp", off: 48, size: 4, ty: "BTConnOperationType", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTDBEntry", size: 52, align: 4, packed: false, declared_in: "AEEBTDevDB.h",
        fields: &[FieldInfo { name: "bBonded", off: 0, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bConnected", off: 1, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "security", off: 4, size: 4, ty: "BTSecurityLevel", bits: 0, bit_off: 0 }, FieldInfo { name: "u16Value1", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "u16Value2", off: 10, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "u16Value3", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "u16Value4", off: 14, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "deviceInfo", off: 16, size: 36, ty: "BTDeviceInfo", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTDevDBEnumerator", size: 26, align: 2, packed: false, declared_in: "AEEBTDevDB.h",
        fields: &[FieldInfo { name: "what2match", off: 0, size: 2, ty: "BTEnumCriteria", bits: 0, bit_off: 0 }, FieldInfo { name: "cos", off: 2, size: 2, ty: "BTClassOfService", bits: 0, bit_off: 0 }, FieldInfo { name: "bBonded", off: 4, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bdAddrMask", off: 5, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "bdAddrValue", off: 11, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "u16Value1", off: 18, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "u16Value2", off: 20, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "u16Value3", off: 22, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "u16Value4", off: 24, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTDevUpdateEvData", size: 12, align: 4, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "bdAddr", off: 0, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "modField", off: 8, size: 4, ty: "BTModDevField", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTDeviceInfo", size: 36, align: 4, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "bdAddr", off: 0, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "LMPFeatures", off: 6, size: 8, ty: "BTLMPFeatures", bits: 0, bit_off: 0 }, FieldInfo { name: "LMPVersion", off: 16, size: 12, ty: "BTLMPVersion", bits: 0, bit_off: 0 }, FieldInfo { name: "COD", off: 28, size: 6, ty: "BTClassOfDevice", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTEventData", size: 12, align: 4, packed: false, declared_in: "AEEBTCommon.h",
        fields: &[FieldInfo { name: "bdAddr", off: 0, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "connEv", off: 0, size: 8, ty: "BTConnEvData", bits: 0, bit_off: 0 }, FieldInfo { name: "devUpdateEv", off: 0, size: 12, ty: "BTDevUpdateEvData", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTLMPFeatures", size: 8, align: 1, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "bytes", off: 0, size: 8, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTLMPVersion", size: 12, align: 4, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "VersNum", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "CompID", off: 4, size: 4, ty: "BTCompanyID", bits: 0, bit_off: 0 }, FieldInfo { name: "SubVersNum", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTLocalInfo", size: 40, align: 4, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "myInfo", off: 0, size: 36, ty: "BTDeviceInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "mySecLevel", off: 36, size: 4, ty: "BTSecurityLevel", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTLocalSettings", size: 80, align: 4, packed: false, declared_in: "AEEIBTMgr.h",
        fields: &[FieldInfo { name: "bdAddr", off: 0, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "u8Name", off: 6, size: 30, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "lmpFeatures", off: 36, size: 8, ty: "BTLMPFeatures", bits: 0, bit_off: 0 }, FieldInfo { name: "lmpVersion", off: 44, size: 12, ty: "BTLMPVersion", bits: 0, bit_off: 0 }, FieldInfo { name: "CoD", off: 56, size: 6, ty: "BTClassOfDevice", bits: 0, bit_off: 0 }, FieldInfo { name: "dSecLevel", off: 64, size: 4, ty: "BTSecurityLevel", bits: 0, bit_off: 0 }, FieldInfo { name: "bRoleSwitchAllowed", off: 68, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bPrefMasterRole", off: 69, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "dPgScanParam", off: 72, size: 4, ty: "BTActVsPwr", bits: 0, bit_off: 0 }, FieldInfo { name: "dInqScanParam", off: 76, size: 4, ty: "BTActVsPwr", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTNotifyInfo", size: 8, align: 4, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "event", off: 0, size: 4, ty: "BTEvent", bits: 0, bit_off: 0 }, FieldInfo { name: "uTimeStamp", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTSPPConfig", size: 5, align: 1, packed: false, declared_in: "AEEBTSPP.h",
        fields: &[FieldInfo { name: "baudRate", off: 0, size: 1, ty: "BTSPPBaudRate", bits: 0, bit_off: 0 }, FieldInfo { name: "format", off: 1, size: 1, ty: "BTSPPDataFormat", bits: 0, bit_off: 0 }, FieldInfo { name: "flowCtrl", off: 2, size: 1, ty: "BTSPPFlowCtrl", bits: 0, bit_off: 0 }, FieldInfo { name: "uXonChar", off: 3, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uXoffChar", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTSPPDeviceInfo", size: 48, align: 4, packed: false, declared_in: "AEEBTSPP.h",
        fields: &[FieldInfo { name: "bdi", off: 0, size: 36, ty: "BTDeviceInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "uNumSPPServices", off: 36, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "serviceRecords", off: 40, size: 8, ty: "BTSPPServiceRecord", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTSPPLineError", size: 20, align: 4, packed: false, declared_in: "AEEBTSPP.h",
        fields: &[FieldInfo { name: "streamID", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "lineError", off: 4, size: 4, ty: "BTSPPLineErrorBitmap", bits: 0, bit_off: 0 }, FieldInfo { name: "uOverRuns", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uParityErrors", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uFramingErrors", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTSPPModemStatus", size: 8, align: 2, packed: false, declared_in: "AEEBTSPP.h",
        fields: &[FieldInfo { name: "streamID", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "status", off: 2, size: 1, ty: "BTSPPModemStatusBitmap", bits: 0, bit_off: 0 }, FieldInfo { name: "changedMask", off: 3, size: 1, ty: "BTSPPModemStatusBitmap", bits: 0, bit_off: 0 }, FieldInfo { name: "breakPresent", off: 4, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "breakLength", off: 5, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "breaks", off: 6, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTSPPServiceRecord", size: 8, align: 4, packed: false, declared_in: "AEEBTSPP.h",
        fields: &[FieldInfo { name: "uuid", off: 0, size: 2, ty: "BTServiceClassUUID", bits: 0, bit_off: 0 }, FieldInfo { name: "uServerChannel", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTSPPStatus", size: 20, align: 4, packed: false, declared_in: "AEEBTSPP.h",
        fields: &[FieldInfo { name: "BDAddr", off: 0, size: 6, ty: "BDAddress", bits: 0, bit_off: 0 }, FieldInfo { name: "streamID", off: 6, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "state", off: 8, size: 1, ty: "AEEBTSPPState", bits: 0, bit_off: 0 }, FieldInfo { name: "result", off: 12, size: 4, ty: "BTResult", bits: 0, bit_off: 0 }, FieldInfo { name: "uChannel", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTServiceAttrib", size: 12, align: 4, packed: false, declared_in: "AEEBTSDDef.h",
        fields: &[FieldInfo { name: "ID", off: 0, size: 4, ty: "BTSvcAttribID", bits: 0, bit_off: 0 }, FieldInfo { name: "type", off: 4, size: 4, ty: "BTSvcAttribType", bits: 0, bit_off: 0 }, FieldInfo { name: "nSize", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTServiceIdentifier", size: 80, align: 4, packed: false, declared_in: "AEEIBTMgr.h",
        fields: &[FieldInfo { name: "dIDMethod", off: 0, size: 4, ty: "BTSvcSecIDMethod", bits: 0, bit_off: 0 }, FieldInfo { name: "nNumItems", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "dProtocol", off: 8, size: 20, ty: "BTProtocol", bits: 0, bit_off: 0 }, FieldInfo { name: "dSvcCls", off: 28, size: 10, ty: "BTServiceClassUUID", bits: 0, bit_off: 0 }, FieldInfo { name: "uSvrChnl", off: 40, size: 20, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uPSM", off: 60, size: 20, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTServiceRecordSummary", size: 8, align: 4, packed: false, declared_in: "AEEBTSDDef.h",
        fields: &[FieldInfo { name: "uHandle", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "profileUUID", off: 4, size: 2, ty: "BTUUID16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTSvcSearchPattern", size: 100, align: 4, packed: false, declared_in: "AEEBTSDDef.h",
        fields: &[FieldInfo { name: "nNumUUID16s", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "UUID16", off: 4, size: 24, ty: "BTUUID16", bits: 0, bit_off: 0 }, FieldInfo { name: "bUUID128Valid", off: 28, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "UUID128", off: 29, size: 16, ty: "BTUUID128", bits: 0, bit_off: 0 }, FieldInfo { name: "nNumAttributes", off: 48, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "attribValue", off: 52, size: 48, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTUUID", size: 8, align: 4, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "dType", off: 0, size: 4, ty: "BTUUIDType", bits: 0, bit_off: 0 }, FieldInfo { name: "uuid", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "BTUUID128", size: 16, align: 1, packed: false, declared_in: "AEEBTDef.h",
        fields: &[FieldInfo { name: "aUUIDByte", off: 0, size: 16, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CBItemInfo", size: 12, align: 4, packed: false, declared_in: "AEEIClipboard.h",
        fields: &[FieldInfo { name: "pszName", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "nNameLen", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nDataSize", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CameraAACEncodingInfo", size: 16, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "uMode", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwFrequency", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uQuality", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwReserved", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CameraData", size: 8, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "pData", off: 0, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBytes", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CameraEncodeUUIDAtomInfo", size: 16, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "pAtom", off: 0, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwAtomSize", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pData", off: 8, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwDataSize", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CameraExifTagInfo", size: 16, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "wTagID", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wTagType", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "pTagData", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBytes", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wIFDID", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wReserved", off: 14, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CameraFocusRectInfo", size: 20, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "dwStructSize", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dxStep", off: 4, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "dyStep", off: 8, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "rectFocusRegion", off: 12, size: 8, ty: "AEERect", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CameraSensorInfo", size: 88, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "dwSize", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "szName", off: 4, size: 64, ty: "AECHAR", bits: 0, bit_off: 0 }, FieldInfo { name: "size", off: 68, size: 8, ty: "AEESize", bits: 0, bit_off: 0 }, FieldInfo { name: "dwFPSPreview", off: 76, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwFPSMovie", off: 80, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "bLandscape", off: 84, size: 4, ty: "flg", bits: 1, bit_off: 0 }],
    },
    StructInfo {
        name: "CameraThumbnailInfo", size: 16, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "nStructSize", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "size", off: 4, size: 8, ty: "AEESize", bits: 0, bit_off: 0 }, FieldInfo { name: "nQuality", off: 12, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CertDataBasic", size: 60, align: 4, packed: false, declared_in: "AEECertData.h",
        fields: &[FieldInfo { name: "uTrustOverrideID", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uSigAlgID", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uStartValidity", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uEndValidity", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uBasicConstraintsPresent", off: 16, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uBasicConstraintsCriticality", off: 17, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uBasicConstraintsIsCa", off: 18, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uBasicConstraintsPathLen", off: 19, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "auExtKeyUse", off: 20, size: 32, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uKeyUse", off: 52, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uKeyUseCriticality", off: 56, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uExtKeyUseCriticality", off: 57, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uCertVersion", off: 58, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CertDataCertPart", size: 24, align: 4, packed: false, declared_in: "AEECertData.h",
        fields: &[FieldInfo { name: "uLength", off: 0, size: 4, ty: "unsigned int", bits: 0, bit_off: 0 }, FieldInfo { name: "pcValue", off: 4, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "pcRawOID", off: 8, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "uASNOID", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "bCriticallity", off: 16, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "uASN1Type", off: 17, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uRDNIndex", off: 18, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "nNumInstances", off: 20, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CertDataIssuerAndSerial", size: 20, align: 4, packed: false, declared_in: "AEECertData.h",
        fields: &[FieldInfo { name: "pcSerial", off: 0, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "nSerialLen", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pcDN", off: 8, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "nDNLen", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "bDNisSubject", off: 16, size: 1, ty: "char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CertDataPartRequest", size: 16, align: 4, packed: false, declared_in: "AEECertData.h",
        fields: &[FieldInfo { name: "uCertPart", off: 0, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "pcRawOID", off: 4, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "uASNOID", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nInstance", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CertDataTrustOverride", size: 8, align: 4, packed: false, declared_in: "AEECertData.h",
        fields: &[FieldInfo { name: "uCertID", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uOverrideBits", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CertDataTrustResult", size: 44, align: 4, packed: false, declared_in: "AEECertData.h",
        fields: &[FieldInfo { name: "nResult", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nChainLen", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "auChainErrors", off: 8, size: 32, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uErrorSummary", off: 40, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CipherInfo", size: 32, align: 4, packed: false, declared_in: "AEEICipherFactory.h",
        fields: &[FieldInfo { name: "cipherId", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "cipherModeId", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "padding", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "encryptDirection", off: 12, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pKey", off: 16, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "cKey", off: 20, size: 4, ty: "unsigned", bits: 0, bit_off: 0 }, FieldInfo { name: "pIV", off: 24, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "cIV", off: 28, size: 4, ty: "unsigned", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CoreAppUIInfo", size: 20, align: 4, packed: false, declared_in: "AEECoreNotifier.h",
        fields: &[FieldInfo { name: "InfoType", off: 0, size: 4, ty: "CoreAppUIInfoType", bits: 0, bit_off: 0 }, FieldInfo { name: "Status", off: 4, size: 4, ty: "CoreAppUIStatusType", bits: 0, bit_off: 0 }, FieldInfo { name: "nErr", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nExtra", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "data", off: 16, size: 4, ty: "CoreAppUIInfo@4", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CoreAppUIInfo@4", size: 4, align: 4, packed: false, declared_in: "AEECoreNotifier.h",
        fields: &[FieldInfo { name: "AppName", off: 0, size: 4, ty: "AECHAR*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CtlAddItem", size: 28, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "pText", off: 0, size: 4, ty: "AECHAR*", bits: 0, bit_off: 0 }, FieldInfo { name: "pImage", off: 4, size: 4, ty: "IImage*", bits: 0, bit_off: 0 }, FieldInfo { name: "pszResImage", off: 8, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "pszResText", off: 12, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "wText", off: 16, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wFont", off: 18, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wImage", off: 20, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wItemID", off: 22, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwData", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "CtlValChange", size: 12, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "pc", off: 0, size: 4, ty: "IControl*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwParam", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "bValid", off: 8, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "DListItem", size: 12, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "wID", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wTextID", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wIconID", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "pad", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwData", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "DialogInfo", size: 56, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "h", off: 0, size: 20, ty: "DialogInfoHead", bits: 0, bit_off: 0 }, FieldInfo { name: "controls", off: 20, size: 36, ty: "DialogItem", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "DialogInfoHead", size: 20, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "wID", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nControls", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "rc", off: 4, size: 8, ty: "AEERect", bits: 0, bit_off: 0 }, FieldInfo { name: "dwProps", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wTitle", off: 16, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wFocusID", off: 18, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "DialogInfoRes", size: 24, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "h", off: 0, size: 20, ty: "DialogInfoHead", bits: 0, bit_off: 0 }, FieldInfo { name: "wControlIDs", off: 20, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "DialogItem", size: 36, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "h", off: 0, size: 24, ty: "DialogItemHead", bits: 0, bit_off: 0 }, FieldInfo { name: "items", off: 24, size: 12, ty: "DListItem", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "DialogItemHead", size: 24, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "cls", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wID", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nItems", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwProps", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "wTextID", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wTitleID", off: 14, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "rc", off: 16, size: 8, ty: "AEERect", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "DialogItemRes", size: 28, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "h", off: 0, size: 24, ty: "DialogItemHead", bits: 0, bit_off: 0 }, FieldInfo { name: "wItemIDs", off: 24, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "EGLCompositeSurfaceCaps", size: 24, align: 4, packed: false, declared_in: "EGLext.h",
        fields: &[FieldInfo { name: "max_overlay", off: 0, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "max_underlay", off: 4, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "max_surface_per_layer", off: 8, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "max_total_surfaces", off: 12, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "pbuffer_support", off: 16, size: 4, ty: "EGLBoolean", bits: 0, bit_off: 0 }, FieldInfo { name: "pixmap_support", off: 20, size: 4, ty: "EGLBoolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "EGLSurfaceScaleCaps", size: 48, align: 4, packed: false, declared_in: "EGLext.h",
        fields: &[FieldInfo { name: "MinXScaleFactor", off: 0, size: 4, ty: "EGLfixed", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxXScaleFactor", off: 4, size: 4, ty: "EGLfixed", bits: 0, bit_off: 0 }, FieldInfo { name: "MinYScaleFactor", off: 8, size: 4, ty: "EGLfixed", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxYScaleFactor", off: 12, size: 4, ty: "EGLfixed", bits: 0, bit_off: 0 }, FieldInfo { name: "MinSrcWidth", off: 16, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxSrcWidth", off: 20, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MinSrcHeight", off: 24, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxSrcHeight", off: 28, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MinDstWidth", off: 32, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxDstWidth", off: 36, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MinDstHeight", off: 40, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "MaxDstHeight", off: 44, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "EGLSurfaceScaleRect", size: 16, align: 4, packed: false, declared_in: "EGLext.h",
        fields: &[FieldInfo { name: "x", off: 0, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "y", off: 4, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "width", off: 8, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }, FieldInfo { name: "height", off: 12, size: 4, ty: "EGLint", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "FSStat", size: 12, align: 4, packed: false, declared_in: "AEEFSStat.h",
        fields: &[FieldInfo { name: "dwMode", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwSize", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwCreationDate", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "FSStatVFS", size: 40, align: 4, packed: false, declared_in: "AEEFSStatVFS.h",
        fields: &[FieldInfo { name: "dwBlockSize", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwFragmentSize", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBlocks", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBlocksFree", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwBlocksAvail", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwNameMax", off: 20, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwFilesUsed", off: 24, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwFSSpaceUsed", off: 28, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwMaxFiles", off: 32, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "dwMaxFSSpace", off: 36, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "GetLine", size: 12, align: 4, packed: false, declared_in: "AEESource.h",
        fields: &[FieldInfo { name: "psz", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "nLen", off: 4, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "bLeftover", off: 8, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "bTruncated", off: 9, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "HMacData", size: 67, align: 1, packed: false, declared_in: "hmac.h",
        fields: &[FieldInfo { name: "bInit", off: 0, size: 1, ty: "unsigned char", bits: 0, bit_off: 0 }, FieldInfo { name: "bIsHMac", off: 1, size: 1, ty: "unsigned char", bits: 0, bit_off: 0 }, FieldInfo { name: "abOPad", off: 2, size: 65, ty: "unsigned char", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "HViewFocus", size: 12, align: 4, packed: false, declared_in: "AEEHtmlViewer.h",
        fields: &[FieldInfo { name: "posElem", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nType", off: 4, size: 4, ty: "HVFocusType", bits: 0, bit_off: 0 }, FieldInfo { name: "nState", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "HViewJump", size: 16, align: 4, packed: false, declared_in: "AEEHtmlViewer.h",
        fields: &[FieldInfo { name: "posElem", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pszURL", off: 4, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "pszMethod", off: 8, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "pszData", off: 12, size: 4, ty: "char*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "HViewNotify", size: 24, align: 4, packed: false, declared_in: "AEEHtmlViewer.h",
        fields: &[FieldInfo { name: "code", off: 0, size: 4, ty: "HVNotifyCode", bits: 0, bit_off: 0 }, FieldInfo { name: "phv", off: 4, size: 4, ty: "IHtmlViewer*", bits: 0, bit_off: 0 }, FieldInfo { name: "u", off: 8, size: 16, ty: "HViewNotify@2", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "HViewNotify@2", size: 16, align: 4, packed: false, declared_in: "AEEHtmlViewer.h",
        fields: &[FieldInfo { name: "jump", off: 0, size: 16, ty: "HViewJump", bits: 0, bit_off: 0 }, FieldInfo { name: "submit", off: 0, size: 16, ty: "HViewJump", bits: 0, bit_off: 0 }, FieldInfo { name: "nDone", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "focus", off: 0, size: 12, ty: "HViewFocus", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IAClockCtl", size: 4, align: 4, packed: false, declared_in: "AEEAClock.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IAClockCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IAStream", size: 4, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "pvt", off: 0, size: 4, ty: "IAStreamVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IAddrBook", size: 4, align: 4, packed: false, declared_in: "AEEAddrBook.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IAddrBookVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IAddrInfo", size: 4, align: 4, packed: false, declared_in: "AEEIAddrInfo.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IAddrInfoVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IAddrInfoCache", size: 4, align: 4, packed: false, declared_in: "AEEIAddrInfoCache.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IAddrInfoCacheVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IAddrRec", size: 4, align: 4, packed: false, declared_in: "AEEAddrBook.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IAddrRecVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IAlarmMgr", size: 4, align: 4, packed: false, declared_in: "AEEAlarmMgr.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IAlarmMgrVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IAppHistory", size: 4, align: 4, packed: false, declared_in: "AEEIAppHistory.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IAppHistoryVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IApplet", size: 4, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "pvt", off: 0, size: 4, ty: "IAppletVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IAppletCtl", size: 4, align: 4, packed: false, declared_in: "AEEIAppletCtl.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IAppletCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBCMCSDB", size: 4, align: 4, packed: false, declared_in: "AEEIBCMCSDB.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBCMCSDBVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBN", size: 4, align: 4, packed: false, declared_in: "AEEIBN.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBNVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBNEng", size: 4, align: 4, packed: false, declared_in: "AEEIBNEng.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBNEngVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBT", size: 4, align: 4, packed: false, declared_in: "AEEBT.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTA2DP", size: 4, align: 4, packed: false, declared_in: "AEEIBTA2DP.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTA2DPVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTA2DPNotifier", size: 4, align: 4, packed: false, declared_in: "AEEIBTA2DPNotifier.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTA2DPNotifierVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTAVRCPTarget", size: 4, align: 4, packed: false, declared_in: "AEEIBTAVRCPTarget.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTAVRCPTargetVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTAudioGateway", size: 4, align: 4, packed: false, declared_in: "AEEIBTAudioGateway.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTAudioGatewayVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTConnection", size: 4, align: 4, packed: false, declared_in: "AEEIBTConnection.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTConnectionVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTDevDB", size: 4, align: 4, packed: false, declared_in: "AEEBTDevDB.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTDevDBVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTMgr", size: 4, align: 4, packed: false, declared_in: "AEEIBTMgr.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTMgrVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTSPP", size: 4, align: 4, packed: false, declared_in: "AEEBTSPP.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTSPPVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTSPPClient", size: 4, align: 4, packed: false, declared_in: "AEEBTSPPClient.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTSPPClientVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTSPPServer", size: 4, align: 4, packed: false, declared_in: "AEEBTSPPServer.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTSPPServerVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBTServiceDiscovery", size: 4, align: 4, packed: false, declared_in: "AEEIBTServiceDiscovery.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBTServiceDiscoveryVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBacklight", size: 4, align: 4, packed: false, declared_in: "AEEBacklight.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBacklightVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBase", size: 4, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "pvt", off: 0, size: 4, ty: "IBaseVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBattery", size: 4, align: 4, packed: false, declared_in: "AEEBattery.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBatteryVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBearerTechnology", size: 4, align: 4, packed: false, declared_in: "AEEIBearerTechnology.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBearerTechnologyVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBitmap", size: 4, align: 4, packed: false, declared_in: "AEEIBitmap.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBitmapVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBitmapDev", size: 4, align: 4, packed: false, declared_in: "AEEIBitmapDev.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBitmapDevVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBitmapDev2", size: 4, align: 4, packed: false, declared_in: "AEEIBitmapDev2.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBitmapDev2Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBitmapRot", size: 4, align: 4, packed: false, declared_in: "AEEBitmapRot.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBitmapRotVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IBitmapScale", size: 4, align: 4, packed: false, declared_in: "AEEIBitmapScale.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IBitmapScaleVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICache", size: 4, align: 4, packed: false, declared_in: "AEERamCache.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICacheVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICall", size: 4, align: 4, packed: false, declared_in: "AEETelephone.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICallVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICallHistory", size: 4, align: 4, packed: false, declared_in: "AEECallHistory.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICallHistoryVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICallMgr", size: 4, align: 4, packed: false, declared_in: "AEETelephone.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICallMgrVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICamera", size: 4, align: 4, packed: false, declared_in: "AEECamera.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICameraVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICertBag", size: 4, align: 4, packed: false, declared_in: "AEEICertBag.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICertBagVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICertChain", size: 4, align: 4, packed: false, declared_in: "AEEICertChain.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICertChainVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICertParse", size: 4, align: 4, packed: false, declared_in: "AEEICertParse.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICertParseVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICharsetConv", size: 4, align: 4, packed: false, declared_in: "AEEICharsetConv.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICharsetConvVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICipher", size: 4, align: 4, packed: false, declared_in: "AEESecurity.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICipherVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICipher1", size: 4, align: 4, packed: false, declared_in: "AEEICipher1.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICipher1Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICipherFactory", size: 4, align: 4, packed: false, declared_in: "AEEICipherFactory.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICipherFactoryVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ICipherWrapper", size: 4, align: 4, packed: false, declared_in: "AEEICipherWrapper.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ICipherWrapperVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IClipboard", size: 4, align: 4, packed: false, declared_in: "AEEIClipboard.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IClipboardVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IControl", size: 4, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IControlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDBMgr", size: 4, align: 4, packed: false, declared_in: "AEEDB.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDBMgrVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDBRecord", size: 4, align: 4, packed: false, declared_in: "AEEDB.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDBRecordVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDIB", size: 36, align: 4, packed: false, declared_in: "AEEIDIB.h",
        fields: &[FieldInfo { name: "pvt", off: 0, size: 4, ty: "AEEVTBL(IBitmap)*", bits: 0, bit_off: 0 }, FieldInfo { name: "pPaletteMap", off: 4, size: 4, ty: "IQI*", bits: 0, bit_off: 0 }, FieldInfo { name: "pBmp", off: 8, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "pRGB", off: 12, size: 4, ty: "uint32*", bits: 0, bit_off: 0 }, FieldInfo { name: "ncTransparent", off: 16, size: 4, ty: "NativeColor", bits: 0, bit_off: 0 }, FieldInfo { name: "cx", off: 20, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 22, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nPitch", off: 24, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "cntRGB", off: 26, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nDepth", off: 28, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "nColorScheme", off: 29, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "reserved", off: 30, size: 6, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDLS", size: 4, align: 4, packed: false, declared_in: "AEEMediaDLS.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDLSVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDLSLinker", size: 4, align: 4, packed: false, declared_in: "AEEMediaDLS.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDLSLinkerVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDNS", size: 4, align: 4, packed: false, declared_in: "AEEIDNS.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDNSVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDNSConfig", size: 4, align: 4, packed: false, declared_in: "AEEIDNSConfig.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDNSConfigVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDNSConfig2", size: 4, align: 4, packed: false, declared_in: "AEEIDNSConfig2.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDNSConfig2Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDatabase", size: 4, align: 4, packed: false, declared_in: "AEEDB.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDatabaseVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDateCtl", size: 4, align: 4, packed: false, declared_in: "AEEDate.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDateCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDialog", size: 4, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDialogVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDispatcher", size: 4, align: 4, packed: false, declared_in: "AEEIDispatcher.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDispatcherVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IDisplay", size: 4, align: 4, packed: false, declared_in: "AEEIDisplay.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IDisplayVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEGL", size: 4, align: 4, packed: false, declared_in: "AEEGL.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEGLVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEGL10", size: 4, align: 4, packed: false, declared_in: "AEEEGL10.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEGL10Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEGL11", size: 4, align: 4, packed: false, declared_in: "AEEEGL11.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEGL11Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEGLGetColorBuffer", size: 4, align: 4, packed: false, declared_in: "AEEEGLGetColorBuffer.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEGLGetColorBufferVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEGLGetColorBufferInfo", size: 4, align: 4, packed: false, declared_in: "AEEEGLGetColorBufferInfo.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEGLGetColorBufferInfoVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEGLGetPowerLevel", size: 4, align: 4, packed: false, declared_in: "AEEEGLGetPowerLevel.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEGLGetPowerLevelVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEGLOESSwapInterval", size: 4, align: 4, packed: false, declared_in: "AEEEGLOESSwapInterval.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEGLOESSwapIntervalVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEGLSurfaceManip", size: 4, align: 4, packed: false, declared_in: "AEEEGLSurfaceManip.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEGLSurfaceManipVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEGLSurfaceManip_V1", size: 4, align: 4, packed: false, declared_in: "AEEEGLSurfaceManip.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEGLSurfaceManip_V1Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IEnv", size: 4, align: 4, packed: false, declared_in: "AEEIEnv.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IEnvVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IFIFO", size: 4, align: 4, packed: false, declared_in: "AEEFIFO.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IFIFOVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IFile", size: 4, align: 4, packed: false, declared_in: "AEEFile.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IFileVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IFileMgr", size: 4, align: 4, packed: false, declared_in: "AEEFile.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IFileMgrVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IFilePort", size: 4, align: 4, packed: false, declared_in: "AEEIFilePort.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IFilePortVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IFileSystem", size: 4, align: 4, packed: false, declared_in: "AEEIFileSystem.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IFileSystemVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IFlip", size: 4, align: 4, packed: false, declared_in: "AEEFlip.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IFlipVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IFont", size: 4, align: 4, packed: false, declared_in: "AEEIFont.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IFontVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IForceFeed", size: 4, align: 4, packed: false, declared_in: "AEEIForceFeed.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IForceFeedVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IGLES10", size: 4, align: 4, packed: false, declared_in: "AEEGLES10.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IGLES10Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IGLES10Ext", size: 4, align: 4, packed: false, declared_in: "AEEGLES10Ext.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IGLES10ExtVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IGLES11", size: 4, align: 4, packed: false, declared_in: "AEEGLES11.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IGLES11Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IGLES11Ext", size: 4, align: 4, packed: false, declared_in: "AEEGLES11Ext.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IGLES11ExtVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IGLES11ExtPak", size: 4, align: 4, packed: false, declared_in: "AEEGLES11ExtPak.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IGLES11ExtPakVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IGLESImageonExt", size: 4, align: 4, packed: false, declared_in: "AEEGLESImageonEXT.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IGLESImageonExtVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IGLESImageonExt_V1", size: 4, align: 4, packed: false, declared_in: "AEEGLESImageonEXT.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IGLESImageonExt_V1Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IGetline", size: 4, align: 4, packed: false, declared_in: "AEESource.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IGetlineVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IGraphics", size: 4, align: 4, packed: false, declared_in: "AEEGraphics.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IGraphicsVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IHID", size: 4, align: 4, packed: false, declared_in: "AEEIHID.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IHIDVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IHIDDevice", size: 4, align: 4, packed: false, declared_in: "AEEIHIDDevice.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IHIDDeviceVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IHID__SeqLongs__seq_long", size: 12, align: 4, packed: false, declared_in: "AEEIHID.h",
        fields: &[FieldInfo { name: "data", off: 0, size: 4, ty: "int*", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLen", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLenReq", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IHash", size: 4, align: 4, packed: false, declared_in: "AEESecurity.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IHashVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IHashCtx", size: 4, align: 4, packed: false, declared_in: "AEEIHashCtx.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IHashCtxVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IHeap", size: 4, align: 4, packed: false, declared_in: "AEEHeap.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IHeapVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IHtmlViewer", size: 4, align: 4, packed: false, declared_in: "AEEHtmlViewer.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IHtmlViewerVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IImage", size: 4, align: 4, packed: false, declared_in: "AEEImage.h",
        fields: &[FieldInfo { name: "pvt", off: 0, size: 4, ty: "IImageVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IImageCtl", size: 4, align: 4, packed: false, declared_in: "AEEImageCtl.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IImageCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IImageDecoder", size: 4, align: 4, packed: false, declared_in: "AEEIImageDecoder.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IImageDecoderVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IItemStore", size: 4, align: 4, packed: false, declared_in: "AEEIItemStore.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IItemStoreVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IJoystick", size: 4, align: 4, packed: false, declared_in: "AEEJoystick.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IJoystickVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ILicense", size: 4, align: 4, packed: false, declared_in: "AEELicense.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ILicenseVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ILocalStorage", size: 4, align: 4, packed: false, declared_in: "AEEILocalStorage.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ILocalStorageVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ILogger", size: 4, align: 4, packed: false, declared_in: "AEELogger.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ILoggerVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IMcastSession", size: 4, align: 4, packed: false, declared_in: "AEEIMcastSession.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IMcastSessionVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IMedia", size: 4, align: 4, packed: false, declared_in: "AEEMedia.h",
        fields: &[FieldInfo { name: "pvt", off: 0, size: 4, ty: "IMediaVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IMediaUtil", size: 4, align: 4, packed: false, declared_in: "AEEMediaUtil.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IMediaUtilVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IMemAStream", size: 4, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IMemAStreamVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IMenuCtl", size: 4, align: 4, packed: false, declared_in: "AEEMenu.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IMenuCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IMod", size: 4, align: 4, packed: false, declared_in: "AEEIMod.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IModVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IModel", size: 4, align: 4, packed: false, declared_in: "AEEIModel.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IModelVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IModule", size: 4, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IModuleVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IMultipartyCall", size: 4, align: 4, packed: false, declared_in: "AEETelephone.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IMultipartyCallVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "INAddr6", size: 16, align: 4, packed: false, declared_in: "AEENetAddrTypes.h",
        fields: &[FieldInfo { name: "Byte", off: 0, size: 16, ty: "byte", bits: 0, bit_off: 0 }, FieldInfo { name: "Word", off: 0, size: 16, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "DWord", off: 0, size: 16, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "INSockAddr", size: 8, align: 4, packed: false, declared_in: "AEENetAddrTypes.h",
        fields: &[FieldInfo { name: "addr", off: 0, size: 4, ty: "INAddr", bits: 0, bit_off: 0 }, FieldInfo { name: "port", off: 4, size: 2, ty: "INPort", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "INetMgr", size: 4, align: 4, packed: false, declared_in: "AEENet.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "INetMgrVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "INetUtils", size: 4, align: 4, packed: false, declared_in: "AEEINetUtils.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "INetUtilsVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "INetwork", size: 4, align: 4, packed: false, declared_in: "AEEINetwork.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "INetworkVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "INotifier", size: 4, align: 4, packed: false, declared_in: "AEE.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "INotifierVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IOBEXCli", size: 4, align: 4, packed: false, declared_in: "AEEIOBEXCli.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IOBEXCliVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IOBEXHeader", size: 4, align: 4, packed: false, declared_in: "AEEIOBEXHeader.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IOBEXHeaderVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IOBEXSrv", size: 4, align: 4, packed: false, declared_in: "AEEIOBEXSrv.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IOBEXSrvVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IOBEXSrvSession", size: 4, align: 4, packed: false, declared_in: "AEEIOBEXSrvSession.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IOBEXSrvSessionVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPAddr", size: 20, align: 4, packed: false, declared_in: "AEENetAddrTypes.h",
        fields: &[FieldInfo { name: "wFamily", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "addr", off: 4, size: 16, ty: "IPAddr@1", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPAddr@1", size: 16, align: 4, packed: false, declared_in: "AEENetAddrTypes.h",
        fields: &[FieldInfo { name: "v4", off: 0, size: 4, ty: "INAddr", bits: 0, bit_off: 0 }, FieldInfo { name: "v6", off: 0, size: 16, ty: "INAddr6", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IParameters", size: 4, align: 4, packed: false, declared_in: "AEEIParameters.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IParametersVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IParameters1", size: 4, align: 4, packed: false, declared_in: "AEEIParameters1.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IParameters1Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IParametersRO", size: 4, align: 4, packed: false, declared_in: "AEEIParameters.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IParametersROVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPeek", size: 4, align: 4, packed: false, declared_in: "AEEIPeek.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IPeekVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPhoneCtl", size: 4, align: 4, packed: false, declared_in: "AEETelephone.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IPhoneCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPort", size: 4, align: 4, packed: false, declared_in: "AEEIPort.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IPortVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPosDet", size: 4, align: 4, packed: false, declared_in: "AEEPosDet.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IPosDetVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPrivSet", size: 4, align: 4, packed: false, declared_in: "AEEIPrivSet.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IPrivSetVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPubKey", size: 4, align: 4, packed: false, declared_in: "AEEIPubKey.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IPubKeyVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPubKeyEng", size: 4, align: 4, packed: false, declared_in: "AEEIPubKeyEng.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IPubKeyEngVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IPubKeyUtil", size: 4, align: 4, packed: false, declared_in: "AEEIPubKeyUtil.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IPubKeyUtilVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IQI", size: 4, align: 4, packed: false, declared_in: "AEEIQI.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IQIVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IQoSBundle", size: 4, align: 4, packed: false, declared_in: "AEEIQoSBundle.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IQoSBundleVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IQoSFilter", size: 4, align: 4, packed: false, declared_in: "AEEIQoSFilter.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IQoSFilterVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IQoSFlow", size: 4, align: 4, packed: false, declared_in: "AEEIQoSFlow.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IQoSFlowVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IQoSList", size: 4, align: 4, packed: false, declared_in: "AEEIQoSList.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IQoSListVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IQoSSession", size: 4, align: 4, packed: false, declared_in: "AEEIQoSSession.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IQoSSessionVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IQoSSpec", size: 4, align: 4, packed: false, declared_in: "AEEIQoSSpec.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IQoSSpecVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IRSA", size: 4, align: 4, packed: false, declared_in: "AEERSA.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IRSAVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IRamCache", size: 4, align: 4, packed: false, declared_in: "AEEIRamCache.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IRamCacheVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IRawBlockCipher", size: 4, align: 4, packed: false, declared_in: "AEEIRawBlockCipher.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IRawBlockCipherVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IRealloc", size: 4, align: 4, packed: false, declared_in: "AEEIRealloc.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IReallocVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IRecordStore", size: 4, align: 4, packed: false, declared_in: "AEEIRecordStore.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IRecordStoreVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IRegistry", size: 4, align: 4, packed: false, declared_in: "AEERegistry.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IRegistryVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IRemoteControl_Target", size: 4, align: 4, packed: false, declared_in: "AEEIRemoteControl_Target.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IRemoteControl_TargetVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IResourceCtl", size: 4, align: 4, packed: false, declared_in: "AEEIResourceCtl.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IResourceCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IRingerMgr", size: 4, align: 4, packed: false, declared_in: "AEERinger.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IRingerMgrVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IRscPool", size: 4, align: 4, packed: false, declared_in: "AEEIRscPool.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IRscPoolVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISMS", size: 4, align: 4, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISMSVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISMSBCConfig", size: 4, align: 4, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISMSBCConfigVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISMSStorage", size: 4, align: 4, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISMSStorageVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISMSStorage2", size: 4, align: 4, packed: false, declared_in: "AEESMS.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISMSStorage2Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISSL", size: 4, align: 4, packed: false, declared_in: "AEESSL.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISSLVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IShell", size: 4, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IShellVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISignal", size: 4, align: 4, packed: false, declared_in: "AEEISignal.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISignalVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISignalBus", size: 4, align: 4, packed: false, declared_in: "AEEISignalBus.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISignalBusVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISignalCBFactory", size: 4, align: 4, packed: false, declared_in: "AEEISignalCBFactory.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISignalCBFactoryVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISignalCtl", size: 4, align: 4, packed: false, declared_in: "AEEISignalCtl.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISignalCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISingleton", size: 4, align: 4, packed: false, declared_in: "AEEISingleton.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISingletonVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISockPort", size: 4, align: 4, packed: false, declared_in: "AEEISockPort.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISockPortVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISockPortDbgMark", size: 8, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "cpszFile", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "nLine", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISocket", size: 4, align: 4, packed: false, declared_in: "AEENet.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISocketVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISocketCloser", size: 8, align: 4, packed: false, declared_in: "AEENet.h",
        fields: &[FieldInfo { name: "pfn", off: 0, size: 4, ty: "PFNSOCKETCLOSE", bits: 0, bit_off: 0 }, FieldInfo { name: "pv", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISocketDbgMark", size: 8, align: 4, packed: false, declared_in: "AEENet.h",
        fields: &[FieldInfo { name: "cpszFile", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "nLine", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISocketQI", size: 8, align: 4, packed: false, declared_in: "AEENet.h",
        fields: &[FieldInfo { name: "clsId", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pif", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISound", size: 4, align: 4, packed: false, declared_in: "AEESound.h",
        fields: &[FieldInfo { name: "pvt", off: 0, size: 4, ty: "AEEVTBL(ISound)*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISoundPlayer", size: 4, align: 4, packed: false, declared_in: "AEESoundPlayer.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISoundPlayerVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISource", size: 4, align: 4, packed: false, declared_in: "AEEISource.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISourceVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISourceUtil", size: 4, align: 4, packed: false, declared_in: "AEESource.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISourceUtilVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISprite", size: 4, align: 4, packed: false, declared_in: "AEESprite.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISpriteVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IStatic", size: 4, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IStaticVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ISuppsTrans", size: 4, align: 4, packed: false, declared_in: "AEETelephone.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ISuppsTransVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ITAPI", size: 4, align: 4, packed: false, declared_in: "AEETAPI.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ITAPIVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ITelephone", size: 4, align: 4, packed: false, declared_in: "AEETelephone.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ITelephoneVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ITextCtl", size: 4, align: 4, packed: false, declared_in: "AEEText.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ITextCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IThread", size: 4, align: 4, packed: false, declared_in: "AEEThread.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IThreadVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ITimeCtl", size: 4, align: 4, packed: false, declared_in: "AEETime.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ITimeCtlVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ITransform", size: 4, align: 4, packed: false, declared_in: "AEEITransform.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "ITransformVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IUnzipAStream", size: 4, align: 4, packed: false, declared_in: "AEEUnzipStream.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IUnzipAStreamVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IVersion", size: 4, align: 4, packed: false, declared_in: "AEEIVersion.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IVersionVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IVocoder", size: 4, align: 4, packed: false, declared_in: "AEEVocoder.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IVocoderVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IVocoderConfigType", size: 44, align: 4, packed: false, declared_in: "AEEVocoder.h",
        fields: &[FieldInfo { name: "usrPtr", off: 0, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "readyCB", off: 4, size: 4, ty: "PFNREADY", bits: 0, bit_off: 0 }, FieldInfo { name: "haveDataCB", off: 8, size: 4, ty: "PFNDATA", bits: 0, bit_off: 0 }, FieldInfo { name: "needDataCB", off: 12, size: 4, ty: "PFNDATA", bits: 0, bit_off: 0 }, FieldInfo { name: "playedDataCB", off: 16, size: 4, ty: "PFNDATA", bits: 0, bit_off: 0 }, FieldInfo { name: "watermark", off: 20, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "txReduction", off: 24, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "max", off: 28, size: 4, ty: "DataRateType", bits: 0, bit_off: 0 }, FieldInfo { name: "min", off: 32, size: 4, ty: "DataRateType", bits: 0, bit_off: 0 }, FieldInfo { name: "vocoder", off: 36, size: 4, ty: "VocoderType", bits: 0, bit_off: 0 }, FieldInfo { name: "overwrite", off: 40, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IVocoderInfoType", size: 4, align: 2, packed: false, declared_in: "AEEVocoder.h",
        fields: &[FieldInfo { name: "frameDuration", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "maxFrameSize", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IWIFI", size: 4, align: 4, packed: false, declared_in: "AEEWIFI.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IWIFIVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IWIFIOpts", size: 4, align: 4, packed: false, declared_in: "AEEWIFI.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IWIFIOptsVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IWeb", size: 4, align: 4, packed: false, declared_in: "AEEWeb.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IWebVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IWebEng", size: 4, align: 4, packed: false, declared_in: "AEEWeb.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IWebEngVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IWebReq", size: 4, align: 4, packed: false, declared_in: "AEEWeb.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IWebReqVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IWebResp", size: 4, align: 4, packed: false, declared_in: "AEEWeb.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IWebRespVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IWebUtil", size: 4, align: 4, packed: false, declared_in: "AEEWeb.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IWebUtilVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IWindowMgr0", size: 4, align: 4, packed: false, declared_in: "AEEIWindowMgr0.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IWindowMgr0Vtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IX509Chain", size: 4, align: 4, packed: false, declared_in: "AEEX509Chain.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IX509ChainVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IvCalObject", size: 4, align: 4, packed: false, declared_in: "AEEIvCalObject.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IvCalObjectVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IvCalStore", size: 4, align: 4, packed: false, declared_in: "AEEIvCalStore.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IvCalStoreVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IvCard", size: 4, align: 4, packed: false, declared_in: "AEEIvCard.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IvCardVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IvObject", size: 4, align: 4, packed: false, declared_in: "AEEIvObject.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IvObjectVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IvObjectParser", size: 4, align: 4, packed: false, declared_in: "AEEIvObjectParser.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IvObjectParserVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IvParm", size: 4, align: 4, packed: false, declared_in: "AEEIvParm.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IvParmVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IvProperty", size: 4, align: 4, packed: false, declared_in: "AEEIvProperty.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IvPropertyVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "IxOpts", size: 4, align: 4, packed: false, declared_in: "AEEIxOpts.h",
        fields: &[FieldInfo { name: "pVtbl", off: 0, size: 4, ty: "IxOptsVtbl*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "KeySupportType", size: 8, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "key", off: 0, size: 4, ty: "AVKType", bits: 0, bit_off: 0 }, FieldInfo { name: "supported", off: 4, size: 1, ty: "boolean", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "LSEntry", size: 16, align: 4, packed: false, declared_in: "AEEILocalStorage.h",
        fields: &[FieldInfo { name: "reserved", off: 0, size: 16, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ModelEvent", size: 12, align: 4, packed: false, declared_in: "AEEIModel.h",
        fields: &[FieldInfo { name: "evCode", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "pModel", off: 4, size: 4, ty: "IModel*", bits: 0, bit_off: 0 }, FieldInfo { name: "dwParam", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "ModelListener", size: 24, align: 4, packed: false, declared_in: "AEEIModel.h",
        fields: &[FieldInfo { name: "pNext", off: 0, size: 4, ty: "ModelListener*", bits: 0, bit_off: 0 }, FieldInfo { name: "pPrev", off: 4, size: 4, ty: "ModelListener*", bits: 0, bit_off: 0 }, FieldInfo { name: "pfnListener", off: 8, size: 4, ty: "PFNLISTENER", bits: 0, bit_off: 0 }, FieldInfo { name: "pListenerData", off: 12, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "pfnCancel", off: 16, size: 4, ty: "PFNLISTENERCANCEL", bits: 0, bit_off: 0 }, FieldInfo { name: "pCancelData", off: 20, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "NotifyKeyEvent", size: 12, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "eCode", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "wParam", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "dwParam", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "clsHandled", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "NotifyStartStatus", size: 8, align: 4, packed: false, declared_in: "AEEShell.h",
        fields: &[FieldInfo { name: "cls", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nErr", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "PortWriteV", size: 12, align: 4, packed: false, declared_in: "AEEPort.h",
        fields: &[FieldInfo { name: "nWrote", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pIOVec", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "wIOVCount", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "PortWriteV_IOVec", size: 8, align: 4, packed: false, declared_in: "AEEPort.h",
        fields: &[FieldInfo { name: "pc", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "wLen", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "SSLCipherMap", size: 8, align: 4, packed: false, declared_in: "AEESSL.h",
        fields: &[FieldInfo { name: "uCryptAlgo", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "uImplCls", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "SSLInfo", size: 60, align: 4, packed: false, declared_in: "AEESSL.h",
        fields: &[FieldInfo { name: "nResult", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "SecInfo", off: 4, size: 8, ty: "SSLSecurityInfo", bits: 0, bit_off: 0 }, FieldInfo { name: "TrustResult", off: 12, size: 44, ty: "X509TrustResult", bits: 0, bit_off: 0 }, FieldInfo { name: "pSSLSocket", off: 56, size: 4, ty: "ISocket*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "SSLSecurityInfo", size: 8, align: 2, packed: false, declared_in: "AEESSL.h",
        fields: &[FieldInfo { name: "uProtoVersion", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "uKeyExchange", off: 2, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uCryptAlgo", off: 3, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uHashAlgo", off: 4, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uCipherSuite", off: 6, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "SVGKeyData", size: 6, align: 2, packed: false, declared_in: "AEEMediaSVG.h",
        fields: &[FieldInfo { name: "nKey", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nVirtualKey", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nModifier", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "SVGPanData", size: 8, align: 4, packed: false, declared_in: "AEEMediaSVG.h",
        fields: &[FieldInfo { name: "x", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "y", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "flgScrollMode", off: 4, size: 4, ty: "flg", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "SVGPointerData", size: 6, align: 2, packed: false, declared_in: "AEEMediaSVG.h",
        fields: &[FieldInfo { name: "x", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "y", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nKeyModifier", off: 4, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "SVGRotateData", size: 12, align: 4, packed: false, declared_in: "AEEMediaSVG.h",
        fields: &[FieldInfo { name: "cx", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "cy", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nAngle", off: 8, size: 2, ty: "int16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "SVGZoomData", size: 12, align: 4, packed: false, declared_in: "AEEMediaSVG.h",
        fields: &[FieldInfo { name: "x", off: 0, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "y", off: 2, size: 2, ty: "int16", bits: 0, bit_off: 0 }, FieldInfo { name: "nZoomFactor", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "flgCentered", off: 8, size: 4, ty: "flg", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "SockIOBlock", size: 8, align: 4, packed: false, declared_in: "AEESocketTypes.h",
        fields: &[FieldInfo { name: "pbBuffer", off: 0, size: 4, ty: "byte*", bits: 0, bit_off: 0 }, FieldInfo { name: "wLen", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "TAPIStatus", size: 8, align: 4, packed: false, declared_in: "AEETAPI.h",
        fields: &[FieldInfo { name: "state", off: 0, size: 1, ty: "AEEPhoneState", bits: 0, bit_off: 0 }, FieldInfo { name: "bData", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 0 }, FieldInfo { name: "bDigital", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 1 }, FieldInfo { name: "bRoaming", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 2 }, FieldInfo { name: "bCallEnded", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 3 }, FieldInfo { name: "bE911CallbackMode", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 4 }, FieldInfo { name: "bRestricted", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 5 }, FieldInfo { name: "bRegistered", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 6 }, FieldInfo { name: "bDormancy", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 7 }, FieldInfo { name: "bHDRMode", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 8 }, FieldInfo { name: "bAirplaneMode", off: 4, size: 4, ty: "flg", bits: 1, bit_off: 9 }],
    },
    StructInfo {
        name: "UrlParts", size: 40, align: 4, packed: false, declared_in: "AEEWeb.h",
        fields: &[FieldInfo { name: "cpcSchm", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpcAuth", off: 4, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpcUser", off: 8, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpcPass", off: 12, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpcHost", off: 16, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpcPort", off: 20, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpcPath", off: 24, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpcSrch", off: 28, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpcName", off: 32, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpcEnd", off: 36, size: 4, ty: "char*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "V3Fx", size: 12, align: 4, packed: false, declared_in: "mathFx.h",
        fields: &[FieldInfo { name: "v", off: 0, size: 12, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "V4Fx", size: 16, align: 4, packed: false, declared_in: "mathFx.h",
        fields: &[FieldInfo { name: "v", off: 0, size: 16, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "WebFormField", size: 8, align: 4, packed: false, declared_in: "AEEWeb.h",
        fields: &[FieldInfo { name: "pcName", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "pcValue", off: 4, size: 4, ty: "char*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "WebRespInfo", size: 28, align: 4, packed: false, declared_in: "AEEWeb.h",
        fields: &[FieldInfo { name: "nCode", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "pisMessage", off: 4, size: 4, ty: "ISource*", bits: 0, bit_off: 0 }, FieldInfo { name: "lContentLength", off: 8, size: 4, ty: "long", bits: 0, bit_off: 0 }, FieldInfo { name: "cpszContentType", off: 12, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "cpszCharset", off: 16, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "tExpires", off: 20, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "tModified", off: 24, size: 4, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "X509BasicCert", size: 36, align: 4, packed: false, declared_in: "AEEX509Chain.h",
        fields: &[FieldInfo { name: "uTrustOverrideID", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uSigAlgID", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uStartValidity", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uEndValidity", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "Key", off: 16, size: 16, ty: "X509RSAKey", bits: 0, bit_off: 0 }, FieldInfo { name: "uVersion", off: 32, size: 1, ty: "uint8", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "X509CertPart", size: 24, align: 4, packed: false, declared_in: "AEEX509Chain.h",
        fields: &[FieldInfo { name: "uLength", off: 0, size: 4, ty: "unsigned int", bits: 0, bit_off: 0 }, FieldInfo { name: "pcValue", off: 4, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "pcRawOID", off: 8, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "uASNOID", off: 12, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "bCriticallity", off: 16, size: 1, ty: "boolean", bits: 0, bit_off: 0 }, FieldInfo { name: "uASN1Type", off: 17, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "uRDNIndex", off: 18, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "nNumInstances", off: 20, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "X509IntegerSet", size: 8, align: 4, packed: false, declared_in: "AEEX509Chain.h",
        fields: &[FieldInfo { name: "uLen", off: 0, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "uOIDLen", off: 2, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "nASNOID", off: 4, size: 4, ty: "int32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "X509PartRequest", size: 24, align: 4, packed: false, declared_in: "AEEX509Chain.h",
        fields: &[FieldInfo { name: "nCertWebOpt", off: 0, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "nCertIndex", off: 4, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "uCertPart", off: 8, size: 1, ty: "uint8", bits: 0, bit_off: 0 }, FieldInfo { name: "pcRawOID", off: 12, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "uASNOID", off: 16, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "nInstance", off: 20, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "X509RSAKey", size: 16, align: 4, packed: false, declared_in: "AEEX509Chain.h",
        fields: &[FieldInfo { name: "puModulus", off: 0, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "uModulusLen", off: 4, size: 2, ty: "uint16", bits: 0, bit_off: 0 }, FieldInfo { name: "puExponent", off: 8, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "uExponentLen", off: 12, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "X509TrustOverride", size: 8, align: 4, packed: false, declared_in: "AEEX509Chain.h",
        fields: &[FieldInfo { name: "uCertID", off: 0, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uOverrideBits", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "X509TrustResult", size: 44, align: 4, packed: false, declared_in: "AEEX509Chain.h",
        fields: &[FieldInfo { name: "nResult", off: 0, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "nChainLen", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "auChainErrors", off: 8, size: 32, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "uErrorSummary", off: 40, size: 4, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "_AEEApplet", size: 32, align: 4, packed: false, declared_in: "AEEAppGen.h",
        fields: &[FieldInfo { name: "vtIApplet", off: 0, size: 4, ty: "IApplet", bits: 0, bit_off: 0 }, FieldInfo { name: "clsID", off: 4, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "m_nRefs", off: 8, size: 4, ty: "uint32", bits: 0, bit_off: 0 }, FieldInfo { name: "m_pIShell", off: 12, size: 4, ty: "IShell*", bits: 0, bit_off: 0 }, FieldInfo { name: "m_pIModule", off: 16, size: 4, ty: "IModule*", bits: 0, bit_off: 0 }, FieldInfo { name: "m_pIDisplay", off: 20, size: 4, ty: "IDisplay*", bits: 0, bit_off: 0 }, FieldInfo { name: "pAppHandleEvent", off: 24, size: 4, ty: "AEEHANDLER", bits: 0, bit_off: 0 }, FieldInfo { name: "pFreeAppData", off: 28, size: 4, ty: "PFNFREEAPPDATA", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "_AEECallback", size: 28, align: 4, packed: false, declared_in: "AEECallback.h",
        fields: &[FieldInfo { name: "pNext", off: 0, size: 4, ty: "AEECallback*", bits: 0, bit_off: 0 }, FieldInfo { name: "pmc", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "pfnCancel", off: 8, size: 4, ty: "CallbackCancelFunc*", bits: 0, bit_off: 0 }, FieldInfo { name: "pCancelData", off: 12, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "pfnNotify", off: 16, size: 4, ty: "CallbackNotifyFunc*", bits: 0, bit_off: 0 }, FieldInfo { name: "pNotifyData", off: 20, size: 4, ty: "void*", bits: 0, bit_off: 0 }, FieldInfo { name: "pReserved", off: 24, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "_SeqLongs__seq_long", size: 12, align: 4, packed: false, declared_in: "AEEIHID.h",
        fields: &[FieldInfo { name: "data", off: 0, size: 4, ty: "int*", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLen", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLenReq", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "__aapcs_va_list", size: 4, align: 4, packed: false, declared_in: "AEEOldVaList.h",
        fields: &[FieldInfo { name: "__ap", off: 0, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "_cstring_t", size: 12, align: 4, packed: false, declared_in: "AEEStdDef.h",
        fields: &[FieldInfo { name: "data", off: 0, size: 4, ty: "char*", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLen", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLenReq", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "_wstring_t", size: 12, align: 4, packed: false, declared_in: "AEEStdDef.h",
        fields: &[FieldInfo { name: "data", off: 0, size: 4, ty: "_wchar_t*", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLen", off: 4, size: 4, ty: "int", bits: 0, bit_off: 0 }, FieldInfo { name: "dataLenReq", off: 8, size: 4, ty: "int", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "dds", size: 8, align: 8, packed: false, declared_in: "AEE64structs.h",
        fields: &[FieldInfo { name: "d", off: 0, size: 8, ty: "double", bits: 0, bit_off: 0 }, FieldInfo { name: "ds", off: 0, size: 8, ty: "doublestruct", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "doublestruct", size: 8, align: 4, packed: false, declared_in: "AEE64structs.h",
        fields: &[FieldInfo { name: "a", off: 0, size: 8, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "iis", size: 8, align: 8, packed: false, declared_in: "AEE64structs.h",
        fields: &[FieldInfo { name: "i", off: 0, size: 8, ty: "int64", bits: 0, bit_off: 0 }, FieldInfo { name: "is", off: 0, size: 8, ty: "int64struct", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "int64struct", size: 8, align: 4, packed: false, declared_in: "AEE64structs.h",
        fields: &[FieldInfo { name: "a", off: 0, size: 8, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "local", size: 12, align: 4, packed: false, declared_in: "AEEBTSPPServerGetObject.h",
        fields: &[FieldInfo { name: "uuid", off: 0, size: 2, ty: "BTServiceClassUUID", bits: 0, bit_off: 0 }, FieldInfo { name: "pszServiceName", off: 4, size: 4, ty: "uint8*", bits: 0, bit_off: 0 }, FieldInfo { name: "version", off: 8, size: 2, ty: "uint16", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "uint64struct", size: 8, align: 4, packed: false, declared_in: "AEE64structs.h",
        fields: &[FieldInfo { name: "a", off: 0, size: 8, ty: "uint32", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "uus", size: 8, align: 8, packed: false, declared_in: "AEE64structs.h",
        fields: &[FieldInfo { name: "u", off: 0, size: 8, ty: "uint64", bits: 0, bit_off: 0 }, FieldInfo { name: "us", off: 0, size: 8, ty: "uint64struct", bits: 0, bit_off: 0 }],
    },
    StructInfo {
        name: "xOpt", size: 8, align: 4, packed: false, declared_in: "AEEIxOpts.h",
        fields: &[FieldInfo { name: "nId", off: 0, size: 4, ty: "int32", bits: 0, bit_off: 0 }, FieldInfo { name: "pVal", off: 4, size: 4, ty: "void*", bits: 0, bit_off: 0 }],
    },
];

pub const SIZE_AEEADDRENUMRECINFO: usize = 40;
pub const SIZE_AEEADDRFIELD: usize = 16;
pub const SIZE_AEEADDRFIELDINFO: usize = 16;
pub const SIZE_AEEADDRINFO: usize = 28;
pub const SIZE_AEEAPPINFO: usize = 20;
pub const SIZE_AEEAPPSTART: usize = 24;
pub const SIZE_AEEAPPUSAGE: usize = 12;
pub const SIZE_AEEARC: usize = 10;
pub const SIZE_AEEAUTOREPEAT: usize = 12;
pub const SIZE_AEEBCMCSDBRECORD_COMPOSITEFLOWID: usize = 16;
pub const SIZE_AEEBCMCSDBRECORD_FLOWDISCRIM: usize = 2;
pub const SIZE_AEEBCMCSDBRECORD_PROGRAMID: usize = 8;
pub const SIZE_AEEBTAVRCPEVENTTYPE: usize = 16;
pub const SIZE_AEEBACKLIGHTINFO: usize = 20;
pub const SIZE_AEEBEARERTECHTYPE: usize = 8;
pub const SIZE_AEEBITMAPINFO: usize = 12;
pub const SIZE_AEECDMABEARERTECHINFO: usize = 4;
pub const SIZE_AEECALLHISTORYENTRY: usize = 8;
pub const SIZE_AEECALLHISTORYFIELD: usize = 12;
pub const SIZE_AEECAMERANOTIFY: usize = 20;
pub const SIZE_AEECARDINFO: usize = 16;
pub const SIZE_AEECIPHERINIT: usize = 32;
pub const SIZE_AEECIRCLE: usize = 6;
pub const SIZE_AEECLIP: usize = 16;
pub const SIZE_AEECONDVAR: usize = 4;
pub const SIZE_AEEDBCOMPLEX: usize = 6;
pub const SIZE_AEEDBFIELD: usize = 16;
pub const SIZE_AEEDBSUBFIELD: usize = 6;
pub const SIZE_AEEDIB: usize = 32;
pub const SIZE_AEEDNSITEM: usize = 20;
pub const SIZE_AEEDNSRESPONSE: usize = 28;
pub const SIZE_AEEDNSRESULT: usize = 20;
pub const SIZE_AEEDATETIME: usize = 14;
pub const SIZE_AEEDEVICEINFO: usize = 64;
pub const SIZE_AEEDEVICENOTIFY: usize = 8;
pub const SIZE_AEEEGLCOMPOSITESURFACECAPS: usize = 24;
pub const SIZE_AEEEGLSURFACESCALECAPS: usize = 48;
pub const SIZE_AEEEGLSURFACESCALERECT: usize = 16;
pub const SIZE_AEEELLIPSE: usize = 8;
pub const SIZE_AEEEXTENT: usize = 8;
pub const SIZE_AEEFSFREESPACE: usize = 16;
pub const SIZE_AEEFILEINFO: usize = 76;
pub const SIZE_AEEFILEINFOEX: usize = 40;
pub const SIZE_AEEFILEUSEINFO: usize = 12;
pub const SIZE_AEEFLIPINFOEX: usize = 16;
pub const SIZE_AEEFONTINFO: usize = 4;
pub const SIZE_AEEGPSCONFIG: usize = 20;
pub const SIZE_AEEGPSIPSERVER: usize = 8;
pub const SIZE_AEEGPSINFO: usize = 44;
pub const SIZE_AEEGPSSERVER: usize = 12;
pub const SIZE_AEEHIDBUTTONINFO: usize = 20;
pub const SIZE_AEEHIDDEVICEINFO: usize = 12;
pub const SIZE_AEEHIDPOSITIONINFO: usize = 100;
pub const SIZE_AEEHIDTHUMBSTICKVALUES: usize = 8;
pub const SIZE_AEEHWIDTYPE: usize = 8;
pub const SIZE_AEEHELPERFUNCS: usize = 468;
pub const SIZE_AEEICMPERRMSG: usize = 148;
pub const SIZE_AEEIPV6PREFIXINFO: usize = 4;
pub const SIZE_AEEIPV6PRIVADDRINFO: usize = 24;
pub const SIZE_AEEIMAGEEXTFUNC: usize = 12;
pub const SIZE_AEEIMAGEINFO: usize = 10;
pub const SIZE_AEEITEMSTYLE: usize = 12;
pub const SIZE_AEEKEYINFOTYPE: usize = 16;
pub const SIZE_AEELINE: usize = 8;
pub const SIZE_AEELINGER: usize = 8;
pub const SIZE_AEEMCASTSESSIONJOINMBMS: usize = 8;
pub const SIZE_AEEMCAST_MBMSJOININFO: usize = 32;
pub const SIZE_AEEMEDIAAUDIOSYNC: usize = 16;
pub const SIZE_AEEMEDIACMDNOTIFY: usize = 28;
pub const SIZE_AEEMEDIACREATEINFO: usize = 20;
pub const SIZE_AEEMEDIADATA: usize = 12;
pub const SIZE_AEEMEDIADATAEX: usize = 36;
pub const SIZE_AEEMEDIADECRYPTINFO: usize = 24;
pub const SIZE_AEEMEDIAENCODEINFO: usize = 20;
pub const SIZE_AEEMEDIAENCODERESULT: usize = 20;
pub const SIZE_AEEMEDIAMIDISPEC: usize = 6;
pub const SIZE_AEEMEDIAMP3SPEC: usize = 144;
pub const SIZE_AEEMEDIASVGSPEC: usize = 144;
pub const SIZE_AEEMEDIAWAVESPEC: usize = 36;
pub const SIZE_AEEMENUCOLORS: usize = 40;
pub const SIZE_AEEMOD: usize = 20;
pub const SIZE_AEEMUTEX: usize = 8;
pub const SIZE_AEENETDOMAINNAME: usize = 256;
pub const SIZE_AEENETSTATS: usize = 32;
pub const SIZE_AEENETUSAGE: usize = 16;
pub const SIZE_AEENETWORKPREFIXINFO: usize = 20;
pub const SIZE_AEENETWORKQOSPROFILES: usize = 12;
pub const SIZE_AEENETWORKSIPSERVERADDRESSES: usize = 12;
pub const SIZE_AEENETWORKSIPSERVERDOMAINNAMES: usize = 12;
pub const SIZE_AEENOTIFY: usize = 20;
pub const SIZE_AEENOTIFYFAILURE: usize = 12;
pub const SIZE_AEEOBEXAUTHRESP: usize = 24;
pub const SIZE_AEEOBEXEVENT: usize = 12;
pub const SIZE_AEEOBEXHEADER: usize = 12;
pub const SIZE_AEEODCBDATA: usize = 20;
pub const SIZE_AEEORIENTATIONAZIMUTHINFO: usize = 8;
pub const SIZE_AEEOUTAGEINFO: usize = 12;
pub const SIZE_AEEPDPINFO: usize = 1;
pub const SIZE_AEEPARMINFO: usize = 20;
pub const SIZE_AEEPIE: usize = 10;
pub const SIZE_AEEPOINT: usize = 4;
pub const SIZE_AEEPOLYGON: usize = 8;
pub const SIZE_AEEPOLYLINE: usize = 8;
pub const SIZE_AEEPOSITIONINFO: usize = 12;
pub const SIZE_AEEPOSITIONINFOEX: usize = 96;
pub const SIZE_AEEPOSITIONINFOEX_ALIGN_32: usize = 92;
pub const SIZE_AEEPROMPTINFO: usize = 40;
pub const SIZE_AEEQOSFILTER_IPV4ADDR: usize = 8;
pub const SIZE_AEEQOSFILTER_IPV4TOS: usize = 2;
pub const SIZE_AEEQOSFILTER_PORT: usize = 4;
pub const SIZE_AEEQOSFLOW_DATARATEMINMAX: usize = 8;
pub const SIZE_AEEQOSFLOW_DATARATETOKENBUCKET: usize = 12;
pub const SIZE_AEEQOSFLOW_NOMINALSDUSIZE: usize = 8;
pub const SIZE_AEEQOSFLOW_PACKETERRORRATE: usize = 4;
pub const SIZE_AEEQOSBUNDLEOPENEX: usize = 12;
pub const SIZE_AEEQOSERROR: usize = 20;
pub const SIZE_AEEQOSSESSIONERROR: usize = 8;
pub const SIZE_AEEQOSSESSIONGETSPECDURINGANYSTATE: usize = 8;
pub const SIZE_AEEQOSSESSSIONMODIFYEX: usize = 8;
pub const SIZE_AEEQOSSESSSIONSELECTNETWORKEX: usize = 8;
pub const SIZE_AEERLP3CFG: usize = 8;
pub const SIZE_AEERECT: usize = 8;
pub const SIZE_AEEREGITEM: usize = 16;
pub const SIZE_AEEREMOTECONTROL_COMMANDTYPE: usize = 12;
pub const SIZE_AEEREMOTECONTROL_VENDORCOMMANDTYPE: usize = 16;
pub const SIZE_AEEREMOTECONTROL___SEQ_UNSIGNED_LONG: usize = 12;
pub const SIZE_AEERESBLOB: usize = 3;
pub const SIZE_AEERINGERCAT: usize = 72;
pub const SIZE_AEERINGERINFO: usize = 140;
pub const SIZE_AEESDBACKCBTYPE: usize = 8;
pub const SIZE_AEESDBACKINFO: usize = 8;
pub const SIZE_AEESIOCONFIG: usize = 20;
pub const SIZE_AEESIOTRIGGERS: usize = 12;
pub const SIZE_AEESMSBCCONFIGEVTDATA: usize = 16;
pub const SIZE_AEESMSBCSRVID: usize = 12;
pub const SIZE_AEESMSCSCHANGEEVTDATA: usize = 12;
pub const SIZE_AEESMSGWTIME: usize = 16;
pub const SIZE_AEESMSMSG: usize = 728;
pub const SIZE_AEESMSSTORAGEEVTDATA: usize = 12;
pub const SIZE_AEESMSSTORAGESTATUS: usize = 16;
pub const SIZE_AEESMSTEXTMSG: usize = 4;
pub const SIZE_AEESMSTIMESTAMP: usize = 6;
pub const SIZE_AEESCREENSAVERINFO: usize = 6;
pub const SIZE_AEESECTORINFO: usize = 24;
pub const SIZE_AEESIZE: usize = 8;
pub const SIZE_AEESOCKADDRINET: usize = 8;
pub const SIZE_AEESOCKADDRINET6: usize = 28;
pub const SIZE_AEESOCKADDRLOCAL: usize = 130;
pub const SIZE_AEESOCKADDRSTORAGE: usize = 130;
pub const SIZE_AEESOCKCMSGHDR: usize = 12;
pub const SIZE_AEESOCKEXTENDEDERR: usize = 16;
pub const SIZE_AEESOCKMSGHDR: usize = 28;
pub const SIZE_AEESOCKOPT: usize = 16;
pub const SIZE_AEESOCKPORTRECVMSG: usize = 8;
pub const SIZE_AEESOCKPORTSELECTNETWORKEX: usize = 8;
pub const SIZE_AEESOUNDCMDDATA: usize = 2;
pub const SIZE_AEESOUNDINFO: usize = 20;
pub const SIZE_AEESOUNDPLAYERAUDIOSPEC: usize = 168;
pub const SIZE_AEESOUNDPLAYERCMDDATA: usize = 168;
pub const SIZE_AEESOUNDPLAYERINFO: usize = 12;
pub const SIZE_AEESOUNDPLAYERMIDISPEC: usize = 4;
pub const SIZE_AEESOUNDPLAYERMP3SPEC: usize = 168;
pub const SIZE_AEESOUNDTONEDATA: usize = 8;
pub const SIZE_AEESPRITECMD: usize = 16;
pub const SIZE_AEESUSPENDINFO: usize = 8;
pub const SIZE_AEESYSERROR: usize = 12;
pub const SIZE_AEETAOCINFO: usize = 12;
pub const SIZE_AEETCALLEVENT: usize = 752;
pub const SIZE_AEETCALLEVENTDATA: usize = 744;
pub const SIZE_AEETCALLINFO: usize = 452;
pub const SIZE_AEETCALLS: usize = 8;
pub const SIZE_AEETCUGINDEX: usize = 3;
pub const SIZE_AEETDETAILEDPLMNINFO: usize = 28;
pub const SIZE_AEETDETAILEDPLMNLIST: usize = 1124;
pub const SIZE_AEETECTINFO: usize = 8;
pub const SIZE_AEETEVENTDATA: usize = 1292;
pub const SIZE_AEETFWDCUGINFO: usize = 6;
pub const SIZE_AEETIS95SYSID: usize = 4;
pub const SIZE_AEETINBANDEVENT: usize = 528;
pub const SIZE_AEETINBANDEVENTDATA: usize = 520;
pub const SIZE_AEETLSAIDENTITY: usize = 8;
pub const SIZE_AEETLINEINFO: usize = 12;
pub const SIZE_AEETMMINFO: usize = 568;
pub const SIZE_AEETNWNAME: usize = 264;
pub const SIZE_AEETNOTIFYINFO: usize = 1296;
pub const SIZE_AEETNUMBER: usize = 108;
pub const SIZE_AEETPLMNID: usize = 3;
pub const SIZE_AEETPHINFO: usize = 1292;
pub const SIZE_AEETPHONEEVENT: usize = 1300;
pub const SIZE_AEETPREFNETWORKS: usize = 62;
pub const SIZE_AEETSSEVENT: usize = 696;
pub const SIZE_AEETSSEVENTDATA: usize = 688;
pub const SIZE_AEETSSINFO: usize = 684;
pub const SIZE_AEETSIGNAL: usize = 16;
pub const SIZE_AEETSIGNALVALUE: usize = 4;
pub const SIZE_AEETSUBADDRESS: usize = 24;
pub const SIZE_AEETSUPPSBASICSERVICEGROUPLISTINFO: usize = 56;
pub const SIZE_AEETSUPPSCCRESPONSE: usize = 524;
pub const SIZE_AEETSUPPSCALLBARRINGFEATURE: usize = 8;
pub const SIZE_AEETSUPPSCALLBARRINGINFO: usize = 112;
pub const SIZE_AEETSUPPSCHANGEPASSWORD: usize = 27;
pub const SIZE_AEETSUPPSCLIRESTRICTINFO: usize = 8;
pub const SIZE_AEETSUPPSDATA: usize = 8;
pub const SIZE_AEETSUPPSDATAEX: usize = 12;
pub const SIZE_AEETSUPPSDATAINFO: usize = 76;
pub const SIZE_AEETSUPPSEVENT: usize = 16;
pub const SIZE_AEETSUPPSEVENTDATA: usize = 8;
pub const SIZE_AEETSUPPSFORWARDINGINFO: usize = 580;
pub const SIZE_AEETSUPPSFORWARDINGOPTION: usize = 8;
pub const SIZE_AEETSUPPSFWDFEATURE: usize = 44;
pub const SIZE_AEETSUPPSFWDFEATURELIST: usize = 576;
pub const SIZE_AEETSUPPSMESSAGE: usize = 44;
pub const SIZE_AEETSUPPSMESSAGEEVENT: usize = 20;
pub const SIZE_AEETSUPPSMESSAGEEVENTDATA: usize = 12;
pub const SIZE_AEETSUPPSNETWORKIND: usize = 12;
pub const SIZE_AEETSUPPSOPSTATUS: usize = 4;
pub const SIZE_AEETSUPPSREGISTRATION: usize = 12;
pub const SIZE_AEETSUPPSREGISTRATIONEX: usize = 20;
pub const SIZE_AEETSUPPSREQUEST: usize = 40;
pub const SIZE_AEETSUPPSRESPONSE: usize = 12;
pub const SIZE_AEETSUPPSRESULT: usize = 588;
pub const SIZE_AEETSUPPSRESULTEVENT: usize = 16;
pub const SIZE_AEETSUPPSRESULTEVENTDATA: usize = 8;
pub const SIZE_AEETSUPPSSTATUS: usize = 592;
pub const SIZE_AEETSUPPSSUBSCRIPTIONOPTION: usize = 8;
pub const SIZE_AEETSYSID: usize = 20;
pub const SIZE_AEETSYSIDDATA: usize = 16;
pub const SIZE_AEETSYSTEMPREFERENCE: usize = 40;
pub const SIZE_AEETTIMEANDZONE: usize = 20;
pub const SIZE_AEETUZNAME: usize = 34;
pub const SIZE_AEETUSERDATAINFO: usize = 132;
pub const SIZE_AEETUSEREVENT: usize = 596;
pub const SIZE_AEETUSEREVENTDATA: usize = 588;
pub const SIZE_AEETUSERPLMNINFO: usize = 6;
pub const SIZE_AEETUSERPLMNLIST: usize = 62;
pub const SIZE_AEETEXT: usize = 8;
pub const SIZE_AEETEXTINPUTMODEINFO: usize = 36;
pub const SIZE_AEETILEMAP: usize = 40;
pub const SIZE_AEETLS: usize = 8;
pub const SIZE_AEETRANSFORMMATRIX: usize = 8;
pub const SIZE_AEETRIANGLE: usize = 12;
pub const SIZE_AEEUDPURGENT: usize = 4;
pub const SIZE_AEEUMTSBEARERTECHINFO: usize = 4;
pub const SIZE_AEEUMTSINFO: usize = 1;
pub const SIZE_AEEVALARM: usize = 20;
pub const SIZE_AEEVDATA: usize = 12;
pub const SIZE_AEEVDATETIMEPERIOD: usize = 32;
pub const SIZE_AEEVGEO: usize = 16;
pub const SIZE_AEEVRRULE: usize = 92;
pub const SIZE_AEEVUTCDATE: usize = 8;
pub const SIZE_AEEVERSIONDESC: usize = 16;
pub const SIZE_AEEVOICEPROMPT: usize = 8;
pub const SIZE_AEEWIFISTATISTICS: usize = 12;
pub const SIZE_AEE_HASH_CTX: usize = 224;
pub const SIZE_AEE_HMAC_MD2_CTX: usize = 216;
pub const SIZE_AEE_HMAC_MD5_CTX: usize = 156;
pub const SIZE_AEE_HMAC_SHA1_CTX: usize = 224;
pub const SIZE_AEE_HMAC_SHA256_CTX: usize = 176;
pub const SIZE_AEE_HMAC_SHA384_CTX: usize = 344;
pub const SIZE_AEE_HMAC_SHA512_CTX: usize = 344;
pub const SIZE_AEE_MD2_CTX: usize = 148;
pub const SIZE_AEE_MD5_CTX: usize = 88;
pub const SIZE_AEE_SHA1_CTX: usize = 156;
pub const SIZE_AEE_SHA256_CTX: usize = 104;
pub const SIZE_AEE_SHA384_CTX: usize = 208;
pub const SIZE_AEE_SHA512_CTX: usize = 208;
pub const SIZE_ASN1OIDMAP: usize = 12;
pub const SIZE_ATITC_HEADER: usize = 20;
pub const SIZE_APPSTARTINFO: usize = 12;
pub const SIZE_BDADDRESS: usize = 6;
pub const SIZE_BTA2DPEVENT: usize = 16;
pub const SIZE_BTAGCONNECTION: usize = 20;
pub const SIZE_BTAGSTATUS: usize = 16;
pub const SIZE_BTCLASSOFDEVICE: usize = 6;
pub const SIZE_BTCONNEVDATA: usize = 8;
pub const SIZE_BTCONNSTATUS: usize = 12;
pub const SIZE_BTCONNECTIONSTATUS: usize = 52;
pub const SIZE_BTDBENTRY: usize = 52;
pub const SIZE_BTDEVDBENUMERATOR: usize = 26;
pub const SIZE_BTDEVUPDATEEVDATA: usize = 12;
pub const SIZE_BTDEVICEINFO: usize = 36;
pub const SIZE_BTEVENTDATA: usize = 12;
pub const SIZE_BTLMPFEATURES: usize = 8;
pub const SIZE_BTLMPVERSION: usize = 12;
pub const SIZE_BTLOCALINFO: usize = 40;
pub const SIZE_BTLOCALSETTINGS: usize = 80;
pub const SIZE_BTNOTIFYINFO: usize = 8;
pub const SIZE_BTSPPCONFIG: usize = 5;
pub const SIZE_BTSPPDEVICEINFO: usize = 48;
pub const SIZE_BTSPPLINEERROR: usize = 20;
pub const SIZE_BTSPPMODEMSTATUS: usize = 8;
pub const SIZE_BTSPPSERVICERECORD: usize = 8;
pub const SIZE_BTSPPSTATUS: usize = 20;
pub const SIZE_BTSERVICEATTRIB: usize = 12;
pub const SIZE_BTSERVICEIDENTIFIER: usize = 80;
pub const SIZE_BTSERVICERECORDSUMMARY: usize = 8;
pub const SIZE_BTSVCSEARCHPATTERN: usize = 100;
pub const SIZE_BTUUID: usize = 8;
pub const SIZE_BTUUID128: usize = 16;
pub const SIZE_CBITEMINFO: usize = 12;
pub const SIZE_CAMERAAACENCODINGINFO: usize = 16;
pub const SIZE_CAMERADATA: usize = 8;
pub const SIZE_CAMERAENCODEUUIDATOMINFO: usize = 16;
pub const SIZE_CAMERAEXIFTAGINFO: usize = 16;
pub const SIZE_CAMERAFOCUSRECTINFO: usize = 20;
pub const SIZE_CAMERASENSORINFO: usize = 88;
pub const SIZE_CAMERATHUMBNAILINFO: usize = 16;
pub const SIZE_CERTDATABASIC: usize = 60;
pub const SIZE_CERTDATACERTPART: usize = 24;
pub const SIZE_CERTDATAISSUERANDSERIAL: usize = 20;
pub const SIZE_CERTDATAPARTREQUEST: usize = 16;
pub const SIZE_CERTDATATRUSTOVERRIDE: usize = 8;
pub const SIZE_CERTDATATRUSTRESULT: usize = 44;
pub const SIZE_CIPHERINFO: usize = 32;
pub const SIZE_COREAPPUIINFO: usize = 20;
pub const SIZE_CTLADDITEM: usize = 28;
pub const SIZE_CTLVALCHANGE: usize = 12;
pub const SIZE_DLISTITEM: usize = 12;
pub const SIZE_DIALOGINFO: usize = 56;
pub const SIZE_DIALOGINFOHEAD: usize = 20;
pub const SIZE_DIALOGINFORES: usize = 24;
pub const SIZE_DIALOGITEM: usize = 36;
pub const SIZE_DIALOGITEMHEAD: usize = 24;
pub const SIZE_DIALOGITEMRES: usize = 28;
pub const SIZE_EGLCOMPOSITESURFACECAPS: usize = 24;
pub const SIZE_EGLSURFACESCALECAPS: usize = 48;
pub const SIZE_EGLSURFACESCALERECT: usize = 16;
pub const SIZE_FSSTAT: usize = 12;
pub const SIZE_FSSTATVFS: usize = 40;
pub const SIZE_GETLINE: usize = 12;
pub const SIZE_HMACDATA: usize = 67;
pub const SIZE_HVIEWFOCUS: usize = 12;
pub const SIZE_HVIEWJUMP: usize = 16;
pub const SIZE_HVIEWNOTIFY: usize = 24;
pub const SIZE_IACLOCKCTL: usize = 4;
pub const SIZE_IASTREAM: usize = 4;
pub const SIZE_IADDRBOOK: usize = 4;
pub const SIZE_IADDRINFO: usize = 4;
pub const SIZE_IADDRINFOCACHE: usize = 4;
pub const SIZE_IADDRREC: usize = 4;
pub const SIZE_IALARMMGR: usize = 4;
pub const SIZE_IAPPHISTORY: usize = 4;
pub const SIZE_IAPPLET: usize = 4;
pub const SIZE_IAPPLETCTL: usize = 4;
pub const SIZE_IBCMCSDB: usize = 4;
pub const SIZE_IBN: usize = 4;
pub const SIZE_IBNENG: usize = 4;
pub const SIZE_IBT: usize = 4;
pub const SIZE_IBTA2DP: usize = 4;
pub const SIZE_IBTA2DPNOTIFIER: usize = 4;
pub const SIZE_IBTAVRCPTARGET: usize = 4;
pub const SIZE_IBTAUDIOGATEWAY: usize = 4;
pub const SIZE_IBTCONNECTION: usize = 4;
pub const SIZE_IBTDEVDB: usize = 4;
pub const SIZE_IBTMGR: usize = 4;
pub const SIZE_IBTSPP: usize = 4;
pub const SIZE_IBTSPPCLIENT: usize = 4;
pub const SIZE_IBTSPPSERVER: usize = 4;
pub const SIZE_IBTSERVICEDISCOVERY: usize = 4;
pub const SIZE_IBACKLIGHT: usize = 4;
pub const SIZE_IBASE: usize = 4;
pub const SIZE_IBATTERY: usize = 4;
pub const SIZE_IBEARERTECHNOLOGY: usize = 4;
pub const SIZE_IBITMAP: usize = 4;
pub const SIZE_IBITMAPDEV: usize = 4;
pub const SIZE_IBITMAPDEV2: usize = 4;
pub const SIZE_IBITMAPROT: usize = 4;
pub const SIZE_IBITMAPSCALE: usize = 4;
pub const SIZE_ICACHE: usize = 4;
pub const SIZE_ICALL: usize = 4;
pub const SIZE_ICALLHISTORY: usize = 4;
pub const SIZE_ICALLMGR: usize = 4;
pub const SIZE_ICAMERA: usize = 4;
pub const SIZE_ICERTBAG: usize = 4;
pub const SIZE_ICERTCHAIN: usize = 4;
pub const SIZE_ICERTPARSE: usize = 4;
pub const SIZE_ICHARSETCONV: usize = 4;
pub const SIZE_ICIPHER: usize = 4;
pub const SIZE_ICIPHER1: usize = 4;
pub const SIZE_ICIPHERFACTORY: usize = 4;
pub const SIZE_ICIPHERWRAPPER: usize = 4;
pub const SIZE_ICLIPBOARD: usize = 4;
pub const SIZE_ICONTROL: usize = 4;
pub const SIZE_IDBMGR: usize = 4;
pub const SIZE_IDBRECORD: usize = 4;
pub const SIZE_IDIB: usize = 36;
pub const SIZE_IDLS: usize = 4;
pub const SIZE_IDLSLINKER: usize = 4;
pub const SIZE_IDNS: usize = 4;
pub const SIZE_IDNSCONFIG: usize = 4;
pub const SIZE_IDNSCONFIG2: usize = 4;
pub const SIZE_IDATABASE: usize = 4;
pub const SIZE_IDATECTL: usize = 4;
pub const SIZE_IDIALOG: usize = 4;
pub const SIZE_IDISPATCHER: usize = 4;
pub const SIZE_IDISPLAY: usize = 4;
pub const SIZE_IEGL: usize = 4;
pub const SIZE_IEGL10: usize = 4;
pub const SIZE_IEGL11: usize = 4;
pub const SIZE_IEGLGETCOLORBUFFER: usize = 4;
pub const SIZE_IEGLGETCOLORBUFFERINFO: usize = 4;
pub const SIZE_IEGLGETPOWERLEVEL: usize = 4;
pub const SIZE_IEGLOESSWAPINTERVAL: usize = 4;
pub const SIZE_IEGLSURFACEMANIP: usize = 4;
pub const SIZE_IEGLSURFACEMANIP_V1: usize = 4;
pub const SIZE_IENV: usize = 4;
pub const SIZE_IFIFO: usize = 4;
pub const SIZE_IFILE: usize = 4;
pub const SIZE_IFILEMGR: usize = 4;
pub const SIZE_IFILEPORT: usize = 4;
pub const SIZE_IFILESYSTEM: usize = 4;
pub const SIZE_IFLIP: usize = 4;
pub const SIZE_IFONT: usize = 4;
pub const SIZE_IFORCEFEED: usize = 4;
pub const SIZE_IGLES10: usize = 4;
pub const SIZE_IGLES10EXT: usize = 4;
pub const SIZE_IGLES11: usize = 4;
pub const SIZE_IGLES11EXT: usize = 4;
pub const SIZE_IGLES11EXTPAK: usize = 4;
pub const SIZE_IGLESIMAGEONEXT: usize = 4;
pub const SIZE_IGLESIMAGEONEXT_V1: usize = 4;
pub const SIZE_IGETLINE: usize = 4;
pub const SIZE_IGRAPHICS: usize = 4;
pub const SIZE_IHID: usize = 4;
pub const SIZE_IHIDDEVICE: usize = 4;
pub const SIZE_IHID__SEQLONGS__SEQ_LONG: usize = 12;
pub const SIZE_IHASH: usize = 4;
pub const SIZE_IHASHCTX: usize = 4;
pub const SIZE_IHEAP: usize = 4;
pub const SIZE_IHTMLVIEWER: usize = 4;
pub const SIZE_IIMAGE: usize = 4;
pub const SIZE_IIMAGECTL: usize = 4;
pub const SIZE_IIMAGEDECODER: usize = 4;
pub const SIZE_IITEMSTORE: usize = 4;
pub const SIZE_IJOYSTICK: usize = 4;
pub const SIZE_ILICENSE: usize = 4;
pub const SIZE_ILOCALSTORAGE: usize = 4;
pub const SIZE_ILOGGER: usize = 4;
pub const SIZE_IMCASTSESSION: usize = 4;
pub const SIZE_IMEDIA: usize = 4;
pub const SIZE_IMEDIAUTIL: usize = 4;
pub const SIZE_IMEMASTREAM: usize = 4;
pub const SIZE_IMENUCTL: usize = 4;
pub const SIZE_IMOD: usize = 4;
pub const SIZE_IMODEL: usize = 4;
pub const SIZE_IMODULE: usize = 4;
pub const SIZE_IMULTIPARTYCALL: usize = 4;
pub const SIZE_INADDR6: usize = 16;
pub const SIZE_INSOCKADDR: usize = 8;
pub const SIZE_INETMGR: usize = 4;
pub const SIZE_INETUTILS: usize = 4;
pub const SIZE_INETWORK: usize = 4;
pub const SIZE_INOTIFIER: usize = 4;
pub const SIZE_IOBEXCLI: usize = 4;
pub const SIZE_IOBEXHEADER: usize = 4;
pub const SIZE_IOBEXSRV: usize = 4;
pub const SIZE_IOBEXSRVSESSION: usize = 4;
pub const SIZE_IPADDR: usize = 20;
pub const SIZE_IPARAMETERS: usize = 4;
pub const SIZE_IPARAMETERS1: usize = 4;
pub const SIZE_IPARAMETERSRO: usize = 4;
pub const SIZE_IPEEK: usize = 4;
pub const SIZE_IPHONECTL: usize = 4;
pub const SIZE_IPORT: usize = 4;
pub const SIZE_IPOSDET: usize = 4;
pub const SIZE_IPRIVSET: usize = 4;
pub const SIZE_IPUBKEY: usize = 4;
pub const SIZE_IPUBKEYENG: usize = 4;
pub const SIZE_IPUBKEYUTIL: usize = 4;
pub const SIZE_IQI: usize = 4;
pub const SIZE_IQOSBUNDLE: usize = 4;
pub const SIZE_IQOSFILTER: usize = 4;
pub const SIZE_IQOSFLOW: usize = 4;
pub const SIZE_IQOSLIST: usize = 4;
pub const SIZE_IQOSSESSION: usize = 4;
pub const SIZE_IQOSSPEC: usize = 4;
pub const SIZE_IRSA: usize = 4;
pub const SIZE_IRAMCACHE: usize = 4;
pub const SIZE_IRAWBLOCKCIPHER: usize = 4;
pub const SIZE_IREALLOC: usize = 4;
pub const SIZE_IRECORDSTORE: usize = 4;
pub const SIZE_IREGISTRY: usize = 4;
pub const SIZE_IREMOTECONTROL_TARGET: usize = 4;
pub const SIZE_IRESOURCECTL: usize = 4;
pub const SIZE_IRINGERMGR: usize = 4;
pub const SIZE_IRSCPOOL: usize = 4;
pub const SIZE_ISMS: usize = 4;
pub const SIZE_ISMSBCCONFIG: usize = 4;
pub const SIZE_ISMSSTORAGE: usize = 4;
pub const SIZE_ISMSSTORAGE2: usize = 4;
pub const SIZE_ISSL: usize = 4;
pub const SIZE_ISHELL: usize = 4;
pub const SIZE_ISIGNAL: usize = 4;
pub const SIZE_ISIGNALBUS: usize = 4;
pub const SIZE_ISIGNALCBFACTORY: usize = 4;
pub const SIZE_ISIGNALCTL: usize = 4;
pub const SIZE_ISINGLETON: usize = 4;
pub const SIZE_ISOCKPORT: usize = 4;
pub const SIZE_ISOCKPORTDBGMARK: usize = 8;
pub const SIZE_ISOCKET: usize = 4;
pub const SIZE_ISOCKETCLOSER: usize = 8;
pub const SIZE_ISOCKETDBGMARK: usize = 8;
pub const SIZE_ISOCKETQI: usize = 8;
pub const SIZE_ISOUND: usize = 4;
pub const SIZE_ISOUNDPLAYER: usize = 4;
pub const SIZE_ISOURCE: usize = 4;
pub const SIZE_ISOURCEUTIL: usize = 4;
pub const SIZE_ISPRITE: usize = 4;
pub const SIZE_ISTATIC: usize = 4;
pub const SIZE_ISUPPSTRANS: usize = 4;
pub const SIZE_ITAPI: usize = 4;
pub const SIZE_ITELEPHONE: usize = 4;
pub const SIZE_ITEXTCTL: usize = 4;
pub const SIZE_ITHREAD: usize = 4;
pub const SIZE_ITIMECTL: usize = 4;
pub const SIZE_ITRANSFORM: usize = 4;
pub const SIZE_IUNZIPASTREAM: usize = 4;
pub const SIZE_IVERSION: usize = 4;
pub const SIZE_IVOCODER: usize = 4;
pub const SIZE_IVOCODERCONFIGTYPE: usize = 44;
pub const SIZE_IVOCODERINFOTYPE: usize = 4;
pub const SIZE_IWIFI: usize = 4;
pub const SIZE_IWIFIOPTS: usize = 4;
pub const SIZE_IWEB: usize = 4;
pub const SIZE_IWEBENG: usize = 4;
pub const SIZE_IWEBREQ: usize = 4;
pub const SIZE_IWEBRESP: usize = 4;
pub const SIZE_IWEBUTIL: usize = 4;
pub const SIZE_IWINDOWMGR0: usize = 4;
pub const SIZE_IX509CHAIN: usize = 4;
pub const SIZE_IVCALOBJECT: usize = 4;
pub const SIZE_IVCALSTORE: usize = 4;
pub const SIZE_IVCARD: usize = 4;
pub const SIZE_IVOBJECT: usize = 4;
pub const SIZE_IVOBJECTPARSER: usize = 4;
pub const SIZE_IVPARM: usize = 4;
pub const SIZE_IVPROPERTY: usize = 4;
pub const SIZE_IXOPTS: usize = 4;
pub const SIZE_KEYSUPPORTTYPE: usize = 8;
pub const SIZE_LSENTRY: usize = 16;
pub const SIZE_MODELEVENT: usize = 12;
pub const SIZE_MODELLISTENER: usize = 24;
pub const SIZE_NOTIFYKEYEVENT: usize = 12;
pub const SIZE_NOTIFYSTARTSTATUS: usize = 8;
pub const SIZE_PORTWRITEV: usize = 12;
pub const SIZE_PORTWRITEV_IOVEC: usize = 8;
pub const SIZE_SSLCIPHERMAP: usize = 8;
pub const SIZE_SSLINFO: usize = 60;
pub const SIZE_SSLSECURITYINFO: usize = 8;
pub const SIZE_SVGKEYDATA: usize = 6;
pub const SIZE_SVGPANDATA: usize = 8;
pub const SIZE_SVGPOINTERDATA: usize = 6;
pub const SIZE_SVGROTATEDATA: usize = 12;
pub const SIZE_SVGZOOMDATA: usize = 12;
pub const SIZE_SOCKIOBLOCK: usize = 8;
pub const SIZE_TAPISTATUS: usize = 8;
pub const SIZE_URLPARTS: usize = 40;
pub const SIZE_V3FX: usize = 12;
pub const SIZE_V4FX: usize = 16;
pub const SIZE_WEBFORMFIELD: usize = 8;
pub const SIZE_WEBRESPINFO: usize = 28;
pub const SIZE_X509BASICCERT: usize = 36;
pub const SIZE_X509CERTPART: usize = 24;
pub const SIZE_X509INTEGERSET: usize = 8;
pub const SIZE_X509PARTREQUEST: usize = 24;
pub const SIZE_X509RSAKEY: usize = 16;
pub const SIZE_X509TRUSTOVERRIDE: usize = 8;
pub const SIZE_X509TRUSTRESULT: usize = 44;
pub const SIZE__AEEAPPLET: usize = 32;
pub const SIZE__AEECALLBACK: usize = 28;
pub const SIZE__SEQLONGS__SEQ_LONG: usize = 12;
pub const SIZE___AAPCS_VA_LIST: usize = 4;
pub const SIZE__CSTRING_T: usize = 12;
pub const SIZE__WSTRING_T: usize = 12;
pub const SIZE_DDS: usize = 8;
pub const SIZE_DOUBLESTRUCT: usize = 8;
pub const SIZE_IIS: usize = 8;
pub const SIZE_INT64STRUCT: usize = 8;
pub const SIZE_LOCAL: usize = 12;
pub const SIZE_UINT64STRUCT: usize = 8;
pub const SIZE_UUS: usize = 8;
pub const SIZE_XOPT: usize = 8;

pub const STRUCT_COUNT: usize = 594;  // 594 of 596 records have a computable size

