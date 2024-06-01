package cyb0124.greg_emitters;

import com.google.gson.Gson;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.sun.jna.*;
import com.sun.jna.platform.linux.Mman;
import com.sun.jna.platform.unix.LibCUtil;
import com.sun.jna.platform.win32.*;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.Base64;
import java.util.Map;

@net.minecraftforge.fml.common.Mod("greg_emitters")
public class Mod {
    public Mod() {
        JsonObject data;
        try (InputStream is = Mod.class.getResourceAsStream("/native.json")) {
            data = new Gson().fromJson(new String(is.readAllBytes(), StandardCharsets.UTF_8), JsonElement.class).getAsJsonObject();
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
        String prefix;
        if (Platform.ARCH.equals("x86-64")) prefix = "x64-";
        else if (Platform.ARCH.equals("aarch64")) prefix = "aarch64-";
        else throw new UnsupportedOperationException("Unsupported architecture: " + Platform.ARCH);
        byte[] blob = Base64.getDecoder().decode(data.get(prefix + "b").getAsString());
        Pointer mem = Platform.isWindows()
                ? Kernel32.INSTANCE.VirtualAllocEx(WinBase.INVALID_HANDLE_VALUE, null, new BaseTSD.SIZE_T(blob.length), WinNT.MEM_COMMIT, WinNT.PAGE_READWRITE)
                : LibCUtil.mmap(null, blob.length, Mman.PROT_READ | Mman.PROT_WRITE, Mman.MAP_PRIVATE | Mman.MAP_ANON, -1, 0);
        mem.write(0, blob, 0, blob.length);
        for (JsonElement item : data.get(prefix + "r").getAsJsonArray()) {
            long r = item.getAsLong();
            mem.setLong((int) (r >> 32), Pointer.nativeValue(mem) + (int) r);
        }
        long exec_len = data.get(prefix + "x").getAsLong() * 4096;
        if (Platform.isWindows()) {
            NativeLibrary.getInstance("kernel32").getFunction("VirtualProtect").invoke(new Object[]{mem, exec_len, WinNT.PAGE_EXECUTE_READ, new WinDef.DWORDByReference()});
            mem = mem.share(data.get(prefix + "w").getAsLong());
        } else {
            NativeLibrary.getInstance("c").getFunction("mprotect").invoke(new Object[]{mem, exec_len, Mman.PROT_READ | Mman.PROT_EXEC});
        }
        Function.getFunction(mem).invoke(Void.class, new Object[]{JNIEnv.CURRENT, this}, Map.of(Library.OPTION_ALLOW_OBJECTS, true));
    }
}
