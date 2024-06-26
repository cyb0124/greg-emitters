use crate::{asm::*, global::GlobalObjs, jvm::*, mapping_base::*, objs, util::UtilExt};
use alloc::{sync::Arc, vec::Vec};
use bstr::B;
use mapping_macros::Functor;

#[derive(Functor)]
pub struct ForgeCN<T> {
    pub cml: T,
    pub naming_domain: T,
    pub fml: T,
    pub fml_java_ctx: T,
    pub evt_bus: T,
    pub reg_keys: T,
    pub reg_evt: T,
    pub forge_reg: T,
    pub non_null_supplier: T,
    pub lazy_opt: T,
    pub cap: T,
    pub cap_provider: T,
    pub container_factory: T,
    pub forge_menu_type: T,
    pub network_hooks: T,
    pub fml_client_setup_evt: T,
    pub network_reg: T,
    pub simple_channel: T,
    pub msg_handler: T,
    pub network_ctx: T,
    pub network_dir: T,
    // Client
    pub renderers_evt: T,
    pub atlas_evt: T,
}

impl ForgeCN<Arc<CSig>> {
    pub fn new() -> Self {
        let names = ForgeCN::<&[u8]> {
            cml: b"cpw.mods.modlauncher.Launcher",
            naming_domain: b"cpw.mods.modlauncher.api.INameMappingService$Domain",
            fml: b"net.minecraftforge.fml.loading.FMLLoader",
            fml_java_ctx: b"net.minecraftforge.fml.javafmlmod.FMLJavaModLoadingContext",
            evt_bus: b"net.minecraftforge.eventbus.api.IEventBus",
            reg_keys: b"net.minecraftforge.registries.ForgeRegistries$Keys",
            reg_evt: b"net.minecraftforge.registries.RegisterEvent",
            forge_reg: b"net.minecraftforge.registries.IForgeRegistry",
            non_null_supplier: b"net.minecraftforge.common.util.NonNullSupplier",
            lazy_opt: b"net.minecraftforge.common.util.LazyOptional",
            cap: b"net.minecraftforge.common.capabilities.Capability",
            cap_provider: b"net.minecraftforge.common.capabilities.CapabilityProvider",
            container_factory: b"net.minecraftforge.network.IContainerFactory",
            forge_menu_type: b"net.minecraftforge.common.extensions.IForgeMenuType",
            network_hooks: b"net.minecraftforge.network.NetworkHooks",
            fml_client_setup_evt: b"net.minecraftforge.fml.event.lifecycle.FMLClientSetupEvent",
            network_reg: b"net.minecraftforge.network.NetworkRegistry",
            simple_channel: b"net.minecraftforge.network.simple.SimpleChannel",
            msg_handler: b"net.minecraftforge.network.simple.IndexedMessageCodec$MessageHandler",
            network_ctx: b"net.minecraftforge.network.NetworkEvent$Context",
            network_dir: b"net.minecraftforge.network.NetworkDirection",
            // Client
            renderers_evt: b"net.minecraftforge.client.event.EntityRenderersEvent$RegisterRenderers",
            atlas_evt: b"net.minecraftforge.client.event.TextureStitchEvent",
        };
        names.fmap(|x| Arc::new(CSig::new(x)))
    }
}

pub struct ForgeMN {
    pub get_cap: MSig,
    pub invalidate_caps: MSig,
    pub non_null_supplier_get: MSig,
    pub container_factory_create: MSig,
    pub forge_menu_type_create: MSig,
    pub network_hooks_open_screen: MSig,
    pub network_reg_new_simple_channel: MSig,
    pub simple_channel_reg_msg: MSig,
    pub gen_enqueue_work: MSig,
}

impl ForgeMN {
    pub fn new(cn: &CN<Arc<CSig>>, fcn: &ForgeCN<Arc<CSig>>) -> Self {
        Self {
            get_cap: MSig {
                owner: fcn.cap_provider.clone(),
                name: cs("getCapability"),
                sig: msig([fcn.cap.sig.to_bytes(), cn.dir.sig.to_bytes()], fcn.lazy_opt.sig.to_bytes()),
            },
            invalidate_caps: MSig { owner: fcn.cap_provider.clone(), name: cs("invalidateCaps"), sig: cs("()V") },
            non_null_supplier_get: MSig { owner: fcn.non_null_supplier.clone(), name: cs("get"), sig: cs("()Ljava/lang/Object;") },
            container_factory_create: MSig {
                owner: fcn.container_factory.clone(),
                name: cs("create"),
                sig: msig([b"I", cn.inventory.sig.to_bytes(), cn.friendly_byte_buf.sig.to_bytes()], cn.container_menu.sig.to_bytes()),
            },
            forge_menu_type_create: MSig {
                owner: fcn.forge_menu_type.clone(),
                name: cs("create"),
                sig: msig([fcn.container_factory.sig.to_bytes()], cn.menu_type.sig.to_bytes()),
            },
            network_hooks_open_screen: MSig {
                owner: fcn.network_hooks.clone(),
                name: cs("openScreen"),
                sig: msig([cn.server_player.sig.to_bytes(), cn.menu_provider.sig.to_bytes(), b"Ljava/util/function/Consumer;"], b"V"),
            },
            network_reg_new_simple_channel: MSig {
                owner: fcn.network_reg.clone(),
                name: cs("newSimpleChannel"),
                sig: msig(
                    [cn.resource_loc.sig.to_bytes(), b"Ljava/util/function/Supplier;Ljava/util/function/Predicate;Ljava/util/function/Predicate;"],
                    fcn.simple_channel.sig.to_bytes(),
                ),
            },
            simple_channel_reg_msg: MSig {
                owner: fcn.simple_channel.clone(),
                name: cs("registerMessage"),
                sig: msig(
                    [B("ILjava/lang/Class;Ljava/util/function/BiConsumer;Ljava/util/function/Function;Ljava/util/function/BiConsumer;")],
                    fcn.msg_handler.sig.to_bytes(),
                ),
            },
            gen_enqueue_work: MSig {
                owner: fcn.network_ctx.clone(),
                name: cs("enqueueWork"),
                sig: cs("(Ljava/lang/Runnable;)Ljava/util/concurrent/CompletableFuture;"),
            },
        }
    }
}

pub struct ForgeMV {
    pub cml_inst: GlobalRef<'static>,
    pub cml_get_mapper: usize,
    pub naming_domain_f: GlobalRef<'static>,
    pub naming_domain_m: GlobalRef<'static>,
    pub fml_naming_is_srg: bool,
    pub mod_evt_bus: GlobalRef<'static>,
    pub evt_bus_add_listener: usize,
    pub reg_key_blocks: GlobalRef<'static>,
    pub reg_key_tile_types: GlobalRef<'static>,
    pub reg_key_menu_types: GlobalRef<'static>,
    pub reg_evt_key: usize,
    pub reg_evt_forge_reg: usize,
    pub forge_reg_reg: usize,
    pub lazy_opt: GlobalRef<'static>,
    pub lazy_opt_of: usize,
    pub lazy_opt_invalidate: usize,
    pub cap_provider: GlobalRef<'static>,
    pub get_cap: usize,
    pub invalidate_caps: usize,
    pub forge_menu_type: GlobalRef<'static>,
    pub forge_menu_type_create: usize,
    pub network_hooks: GlobalRef<'static>,
    pub network_hooks_open_screen: usize,
    pub parallel_dispatch_evt_enqueue: usize,
    pub network_reg: GlobalRef<'static>,
    pub network_reg_new_simple_channel: usize,
    pub simple_channel_reg_msg: usize,
    pub simple_channel_send_to_server: usize,
    pub network_ctx_set_handled: usize,
    pub network_ctx_get_dir: usize,
    pub network_ctx_get_sender: usize,
    pub network_ctx_enqueue_task: usize,
    pub network_dir_s2c: GlobalRef<'static>,
    pub network_dir_c2s: GlobalRef<'static>,
    pub client: Option<ForgeMVC>,
}

pub struct ForgeMVC {
    pub renderers_evt_reg: usize,
    pub atlas_evt_get_atlas: usize,
}

impl ForgeMV {
    pub fn new(av: &AV<'static>, cn: &CN<Arc<CSig>>, fcn: &ForgeCN<Arc<CSig>>, fmn: &ForgeMN) -> Self {
        let load = |csig: &Arc<CSig>| av.ldr.load_class(&av.jv, &csig.dot).unwrap().new_global_ref().unwrap();
        let cml = load(&fcn.cml);
        let naming_domain = load(&fcn.naming_domain);
        let fml = av.ldr.load_class(&av.jv, c"net.minecraftforge.fml.loading.FMLLoader").unwrap();
        let fml_naming = fml.get_static_field_id(c"naming", c"Ljava/lang/String;").unwrap();
        let fml_naming_is_srg = &*fml.get_static_object_field(fml_naming).unwrap().utf_chars().unwrap() == b"srg";
        let fml_java_ctx = load(&fcn.fml_java_ctx);
        let fml_ctx_inst = fml_java_ctx.get_static_method_id(c"get", &msig([], fcn.fml_java_ctx.sig.to_bytes())).unwrap();
        let fml_ctx_inst = fml_java_ctx.call_static_object_method(fml_ctx_inst, &[]).unwrap().unwrap();
        let evt_bus = load(&fcn.evt_bus);
        let mod_evt_bus = fml_java_ctx.get_method_id(c"getModEventBus", &msig([], fcn.evt_bus.sig.to_bytes())).unwrap();
        let mod_evt_bus = fml_ctx_inst.call_object_method(mod_evt_bus, &[]).unwrap().unwrap().new_global_ref().unwrap();
        let reg_keys = load(&fcn.reg_keys);
        let reg_evt = load(&fcn.reg_evt);
        let lazy_opt = load(&fcn.lazy_opt);
        let cap_provider = load(&fcn.cap_provider);
        let forge_menu_type = load(&fcn.forge_menu_type);
        let network_hooks = load(&fcn.network_hooks);
        let parallel_dispatch_evt = av.ldr.load_class(&av.jv, c"net.minecraftforge.fml.event.lifecycle.ParallelDispatchEvent").unwrap();
        let network_reg = load(&fcn.network_reg);
        let simple_channel = load(&fcn.simple_channel);
        let network_ctx = load(&fcn.network_ctx);
        let network_dir = load(&fcn.network_dir);
        let dist = fml.static_field_1(c"dist", c"Lnet/minecraftforge/api/distmarker/Dist;");
        let is_client = dist.call_bool_method(dist.get_object_class().get_method_id(c"isClient", c"()Z").unwrap(), &[]).unwrap();
        Self {
            cml_inst: cml.static_field_1(c"INSTANCE", &fcn.cml.sig),
            cml_get_mapper: cml.get_method_id(c"findNameMapping", c"(Ljava/lang/String;)Ljava/util/Optional;").unwrap(),
            naming_domain_f: naming_domain.static_field_1(c"FIELD", &fcn.naming_domain.sig),
            naming_domain_m: naming_domain.static_field_1(c"METHOD", &fcn.naming_domain.sig),
            fml_naming_is_srg,
            mod_evt_bus,
            evt_bus_add_listener: evt_bus.get_method_id(c"addListener", c"(Ljava/util/function/Consumer;)V").unwrap(),
            reg_key_blocks: reg_keys.static_field_1(c"BLOCKS", &cn.resource_key.sig),
            reg_key_tile_types: reg_keys.static_field_1(c"BLOCK_ENTITY_TYPES", &cn.resource_key.sig),
            reg_key_menu_types: reg_keys.static_field_1(c"MENU_TYPES", &cn.resource_key.sig),
            reg_evt_key: reg_evt.get_method_id(c"getRegistryKey", &msig([], cn.resource_key.sig.to_bytes())).unwrap(),
            reg_evt_forge_reg: reg_evt.get_method_id(c"getForgeRegistry", &msig([], fcn.forge_reg.sig.to_bytes())).unwrap(),
            forge_reg_reg: load(&fcn.forge_reg).get_method_id(c"register", c"(Ljava/lang/String;Ljava/lang/Object;)V").unwrap(),
            lazy_opt_of: lazy_opt.get_static_method_id(c"of", &msig([fcn.non_null_supplier.sig.to_bytes()], fcn.lazy_opt.sig.to_bytes())).unwrap(),
            lazy_opt_invalidate: lazy_opt.get_method_id(c"invalidate", c"()V").unwrap(),
            lazy_opt,
            get_cap: fmn.get_cap.get_method_id(&cap_provider).unwrap(),
            invalidate_caps: fmn.invalidate_caps.get_method_id(&cap_provider).unwrap(),
            cap_provider,
            forge_menu_type_create: fmn.forge_menu_type_create.get_static_method_id(&forge_menu_type).unwrap(),
            forge_menu_type,
            network_hooks_open_screen: fmn.network_hooks_open_screen.get_static_method_id(&network_hooks).unwrap(),
            network_hooks,
            parallel_dispatch_evt_enqueue: fmn.gen_enqueue_work.get_method_id(&parallel_dispatch_evt).unwrap(),
            network_reg_new_simple_channel: fmn.network_reg_new_simple_channel.get_static_method_id(&network_reg).unwrap(),
            network_reg,
            simple_channel_reg_msg: fmn.simple_channel_reg_msg.get_method_id(&simple_channel).unwrap(),
            simple_channel_send_to_server: simple_channel.get_method_id(c"sendToServer", c"(Ljava/lang/Object;)V").unwrap(),
            network_ctx_set_handled: network_ctx.get_method_id(c"setPacketHandled", c"(Z)V").unwrap(),
            network_ctx_get_dir: network_ctx.get_method_id(c"getDirection", &msig([], fcn.network_dir.sig.to_bytes())).unwrap(),
            network_ctx_get_sender: network_ctx.get_method_id(c"getSender", &msig([], cn.server_player.sig.to_bytes())).unwrap(),
            network_ctx_enqueue_task: fmn.gen_enqueue_work.get_method_id(&network_ctx).unwrap(),
            network_dir_s2c: network_dir.static_field_1(c"PLAY_TO_CLIENT", &fcn.network_dir.sig),
            network_dir_c2s: network_dir.static_field_1(c"PLAY_TO_SERVER", &fcn.network_dir.sig),
            client: is_client.then(|| {
                let renderers_evt = load(&fcn.renderers_evt);
                let renderers_evt_reg = msig([cn.tile_type.sig.to_bytes(), cn.tile_renderer_provider.sig.to_bytes()], b"V");
                let atlas_evt = load(&fcn.atlas_evt);
                ForgeMVC {
                    renderers_evt_reg: renderers_evt.get_method_id(c"registerBlockEntityRenderer", &renderers_evt_reg).unwrap(),
                    atlas_evt_get_atlas: atlas_evt.get_method_id(c"getAtlas", &msig([], cn.atlas.sig.to_bytes())).unwrap(),
                }
            }),
        }
    }
}

#[derive(Functor)]
pub struct CN<T> {
    pub block: T,
    pub block_beh: T,
    pub block_beh_props: T,
    pub block_getter: T,
    pub blocks: T,
    pub level: T,
    pub base_tile_block: T,
    pub tile_block: T,
    pub resource_key: T,
    pub item: T,
    pub item_props: T,
    pub item_stack: T,
    pub item_like: T,
    pub block_item: T,
    pub tile: T,
    pub tile_supplier: T,
    pub tile_type: T,
    pub vec3i: T,
    pub block_pos: T,
    pub block_state: T,
    pub dfu_type: T,
    pub sound_type: T,
    pub creative_tab_items_gen: T,
    pub creative_tab_params: T,
    pub creative_tab_output: T,
    pub render_shape: T,
    pub resource_loc: T,
    pub voxel_shape: T,
    pub shapes: T,
    pub collision_ctx: T,
    pub block_place_ctx: T,
    pub use_on_ctx: T,
    pub interaction_result: T,
    pub s2c_tile_data: T,
    pub nbt_compound: T,
    pub packet: T,
    pub living_entity: T,
    pub dir: T,
    pub loot_builder: T,
    pub container_menu: T,
    pub inventory: T,
    pub friendly_byte_buf: T,
    pub menu_type: T,
    pub server_player: T,
    pub menu_provider: T,
    pub player: T,
    pub chat_component: T,
    pub chat_mutable_component: T,
    pub formatted_char_seq: T,
    pub interaction_hand: T,
    pub block_hit_result: T,
    pub entity: T,
    pub container: T,
    pub game_profile: T,
    pub holder: T,
    pub holder_ref: T,
    pub sound_evts: T,
    pub chunk_source: T,
    pub server_chunk_cache: T,
    // Client
    pub tile_renderer: T,
    pub tile_renderer_provider: T,
    pub tile_renderer_provider_ctx: T,
    pub pose_stack: T,
    pub pose: T,
    pub matrix4f: T,
    pub matrix4fc: T,
    pub atlas: T,
    pub sprite: T,
    pub sheets: T,
    pub multi_buffer_source: T,
    pub vertex_consumer: T,
    pub render_type: T,
    pub menu_screens: T,
    pub screen_constructor: T,
    pub container_screen: T,
    pub screen: T,
    pub gui_graphics: T,
    pub font: T,
    pub game_renderer: T,
    pub render_sys: T,
    pub shader_inst: T,
    pub tesselator: T,
    pub vertex_mode: T,
    pub vertex_fmt: T,
    pub default_vertex_fmt: T,
    pub buffer_builder: T,
    pub mc: T,
    pub window: T,
    pub sound_mgr: T,
    pub sound_inst: T,
    pub simple_sound_inst: T,
}

impl CN<Arc<CSig>> {
    pub fn new() -> Self {
        let names = CN::<&[u8]> {
            block: b"net.minecraft.world.level.block.Block",
            block_beh: b"net.minecraft.world.level.block.state.BlockBehaviour",
            block_beh_props: b"net.minecraft.world.level.block.state.BlockBehaviour$Properties",
            block_getter: b"net.minecraft.world.level.BlockGetter",
            blocks: b"net.minecraft.world.level.block.Blocks",
            level: b"net.minecraft.world.level.Level",
            base_tile_block: b"net.minecraft.world.level.block.BaseEntityBlock",
            tile_block: b"net.minecraft.world.level.block.EntityBlock",
            resource_key: b"net.minecraft.resources.ResourceKey",
            item: b"net.minecraft.world.item.Item",
            item_props: b"net.minecraft.world.item.Item$Properties",
            item_stack: b"net.minecraft.world.item.ItemStack",
            item_like: b"net.minecraft.world.level.ItemLike",
            block_item: b"net.minecraft.world.item.BlockItem",
            tile: b"net.minecraft.world.level.block.entity.BlockEntity",
            tile_supplier: b"net.minecraft.world.level.block.entity.BlockEntityType$BlockEntitySupplier",
            tile_type: b"net.minecraft.world.level.block.entity.BlockEntityType",
            vec3i: b"net.minecraft.core.Vec3i",
            block_pos: b"net.minecraft.core.BlockPos",
            block_state: b"net.minecraft.world.level.block.state.BlockState",
            dfu_type: b"com.mojang.datafixers.types.Type",
            sound_type: b"net.minecraft.world.level.block.SoundType",
            voxel_shape: b"net.minecraft.world.phys.shapes.VoxelShape",
            shapes: b"net.minecraft.world.phys.shapes.Shapes",
            creative_tab_items_gen: b"net.minecraft.world.item.CreativeModeTab$DisplayItemsGenerator",
            creative_tab_params: b"net.minecraft.world.item.CreativeModeTab$ItemDisplayParameters",
            creative_tab_output: b"net.minecraft.world.item.CreativeModeTab$Output",
            render_shape: b"net.minecraft.world.level.block.RenderShape",
            resource_loc: b"net.minecraft.resources.ResourceLocation",
            collision_ctx: b"net.minecraft.world.phys.shapes.CollisionContext",
            block_place_ctx: b"net.minecraft.world.item.context.BlockPlaceContext",
            use_on_ctx: b"net.minecraft.world.item.context.UseOnContext",
            interaction_result: b"net.minecraft.world.InteractionResult",
            s2c_tile_data: b"net.minecraft.network.protocol.game.ClientboundBlockEntityDataPacket",
            nbt_compound: b"net.minecraft.nbt.CompoundTag",
            packet: b"net.minecraft.network.protocol.Packet",
            living_entity: b"net.minecraft.world.entity.LivingEntity",
            dir: b"net.minecraft.core.Direction",
            loot_builder: b"net.minecraft.world.level.storage.loot.LootParams$Builder",
            container_menu: b"net.minecraft.world.inventory.AbstractContainerMenu",
            inventory: b"net.minecraft.world.entity.player.Inventory",
            friendly_byte_buf: b"net.minecraft.network.FriendlyByteBuf",
            menu_type: b"net.minecraft.world.inventory.MenuType",
            server_player: b"net.minecraft.server.level.ServerPlayer",
            menu_provider: b"net.minecraft.world.MenuProvider",
            player: b"net.minecraft.world.entity.player.Player",
            chat_component: b"net.minecraft.network.chat.Component",
            chat_mutable_component: b"net.minecraft.network.chat.MutableComponent",
            formatted_char_seq: b"net.minecraft.util.FormattedCharSequence",
            interaction_hand: b"net.minecraft.world.InteractionHand",
            block_hit_result: b"net.minecraft.world.phys.BlockHitResult",
            entity: b"net.minecraft.world.entity.Entity",
            container: b"net.minecraft.world.Container",
            game_profile: b"com.mojang.authlib.GameProfile",
            holder: b"net.minecraft.core.Holder",
            holder_ref: b"net.minecraft.core.Holder$Reference",
            sound_evts: b"net.minecraft.sounds.SoundEvents",
            chunk_source: b"net.minecraft.world.level.chunk.ChunkSource",
            server_chunk_cache: b"net.minecraft.server.level.ServerChunkCache",
            // Client
            tile_renderer: b"net.minecraft.client.renderer.blockentity.BlockEntityRenderer",
            tile_renderer_provider: b"net.minecraft.client.renderer.blockentity.BlockEntityRendererProvider",
            tile_renderer_provider_ctx: b"net.minecraft.client.renderer.blockentity.BlockEntityRendererProvider$Context",
            pose_stack: b"com.mojang.blaze3d.vertex.PoseStack",
            pose: b"com.mojang.blaze3d.vertex.PoseStack$Pose",
            matrix4f: b"org.joml.Matrix4f",
            matrix4fc: b"org.joml.Matrix4fc",
            atlas: b"net.minecraft.client.renderer.texture.TextureAtlas",
            sprite: b"net.minecraft.client.renderer.texture.TextureAtlasSprite",
            sheets: b"net.minecraft.client.renderer.Sheets",
            multi_buffer_source: b"net.minecraft.client.renderer.MultiBufferSource",
            vertex_consumer: b"com.mojang.blaze3d.vertex.VertexConsumer",
            render_type: b"net.minecraft.client.renderer.RenderType",
            menu_screens: b"net.minecraft.client.gui.screens.MenuScreens",
            screen_constructor: b"net.minecraft.client.gui.screens.MenuScreens$ScreenConstructor",
            container_screen: b"net.minecraft.client.gui.screens.inventory.AbstractContainerScreen",
            screen: b"net.minecraft.client.gui.screens.Screen",
            gui_graphics: b"net.minecraft.client.gui.GuiGraphics",
            font: b"net.minecraft.client.gui.Font",
            game_renderer: b"net.minecraft.client.renderer.GameRenderer",
            render_sys: b"com.mojang.blaze3d.systems.RenderSystem",
            shader_inst: b"net.minecraft.client.renderer.ShaderInstance",
            tesselator: b"com.mojang.blaze3d.vertex.Tesselator",
            vertex_mode: b"com.mojang.blaze3d.vertex.VertexFormat$Mode",
            vertex_fmt: b"com.mojang.blaze3d.vertex.VertexFormat",
            default_vertex_fmt: b"com.mojang.blaze3d.vertex.DefaultVertexFormat",
            buffer_builder: b"com.mojang.blaze3d.vertex.BufferBuilder",
            mc: b"net.minecraft.client.Minecraft",
            window: b"com.mojang.blaze3d.platform.Window",
            sound_mgr: b"net.minecraft.client.sounds.SoundManager",
            sound_inst: b"net.minecraft.client.resources.sounds.SoundInstance",
            simple_sound_inst: b"net.minecraft.client.resources.sounds.SimpleSoundInstance",
        };
        names.fmap(|x| Arc::new(CSig::new(x)))
    }
}

#[derive(Functor)]
pub struct MN<T> {
    pub base_tile_block_init: T,
    pub tile_block_new_tile: T,
    pub block_default_state: T,
    pub block_beh_props_of: T,
    pub block_beh_props_strength: T,
    pub block_beh_props_dyn_shape: T,
    pub block_beh_props_sound: T,
    pub block_beh_get_render_shape: T,
    pub block_beh_get_shape: T,
    pub block_beh_get_drops: T,
    pub block_beh_on_place: T,
    pub block_beh_use: T,
    pub block_item_init: T,
    pub block_item_place_block: T,
    pub block_getter_get_block_state: T,
    pub block_getter_get_tile: T,
    pub vec3i_x: T,
    pub vec3i_y: T,
    pub vec3i_z: T,
    pub block_pos_init: T,
    pub block_state_get_block: T,
    pub blocks_fire: T,
    pub tile_supplier_create: T,
    pub tile_type_init: T,
    pub tile_init: T,
    pub tile_load: T,
    pub tile_get_update_tag: T,
    pub tile_get_update_packet: T,
    pub tile_save_additional: T,
    pub tile_level: T,
    pub tile_pos: T,
    pub sound_type_metal: T,
    pub item_get_desc_id: T,
    pub item_stack_init: T,
    pub creative_tab_items_gen_accept: T,
    pub render_shape_tile: T,
    pub resource_loc_init: T,
    pub shapes_create: T,
    pub s2c_tile_data_create: T,
    pub nbt_compound_init: T,
    pub nbt_compound_put_byte_array: T,
    pub nbt_compound_get_byte_array: T,
    pub use_on_ctx_get_level: T,
    pub use_on_ctx_get_clicked_pos: T,
    pub use_on_ctx_get_clicked_face: T,
    pub dir_3d_data: T,
    pub dir_by_3d_data: T,
    pub level_set_block_and_update: T,
    pub level_update_neighbors_for_out_signal: T,
    pub level_is_client: T,
    pub level_get_chunk_source: T,
    pub container_menu_init: T,
    pub container_menu_still_valid: T,
    pub container_menu_quick_move_stack: T,
    pub container_menu_id: T,
    pub menu_provider_create_menu: T,
    pub menu_provider_get_display_name: T,
    pub chat_component_translatable: T,
    pub chat_component_literal: T,
    pub chat_component_to_formatted: T,
    pub friendly_byte_buf_read_byte_array: T,
    pub friendly_byte_buf_write_byte_array: T,
    pub inventory_player: T,
    pub entity_level: T,
    pub container_still_valid: T,
    pub player_profile: T,
    pub player_container_menu: T,
    pub game_profile_get_name: T,
    pub sound_evts_ui_btn_click: T,
    pub server_chunk_cache_block_changed: T,
    // Client
    pub tile_renderer_render: T,
    pub tile_renderer_provider_create: T,
    pub pose_pose: T,
    pub pose_stack_last: T,
    pub matrix4fc_read: T,
    pub atlas_loc: T,
    pub atlas_loc_blocks: T,
    pub atlas_get_sprite: T,
    pub sprite_u0: T,
    pub sprite_v0: T,
    pub sprite_u1: T,
    pub sprite_v1: T,
    pub sheets_solid: T,
    pub buffer_source_get_buffer: T,
    pub vertex_consumer_vertex: T,
    pub vertex_consumer_pos: T,
    pub vertex_consumer_color: T,
    pub vertex_consumer_end_vertex: T,
    pub menu_screens_reg: T,
    pub screen_constructor_create: T,
    pub screen_title: T,
    pub screen_font: T,
    pub screen_width: T,
    pub screen_height: T,
    pub screen_render_background: T,
    pub container_screen_init: T,
    pub container_screen_minit: T,
    pub container_screen_img_width: T,
    pub container_screen_img_height: T,
    pub container_screen_title_x: T,
    pub container_screen_title_y: T,
    pub container_screen_left: T,
    pub container_screen_top: T,
    pub container_screen_menu: T,
    pub container_screen_render_bg: T,
    pub container_screen_render_labels: T,
    pub container_screen_mouse_clicked: T,
    pub container_screen_mouse_dragged: T,
    pub container_screen_mouse_released: T,
    pub gui_graphics_draw_formatted: T,
    pub gui_graphics_pose: T,
    pub render_sys_set_shader: T,
    pub render_sys_enable_blend: T,
    pub render_sys_enable_cull: T,
    pub render_sys_disable_blend: T,
    pub render_sys_disable_cull: T,
    pub game_renderer_get_pos_color_shader: T,
    pub tesselator_get_inst: T,
    pub tesselator_get_builder: T,
    pub tesselator_end: T,
    pub vertex_mode_tris: T,
    pub default_vertex_fmt_pos_color: T,
    pub buffer_builder_begin: T,
    pub mc_get_inst: T,
    pub mc_get_window: T,
    pub mc_get_sound_mgr: T,
    pub window_get_gui_scale: T,
    pub font_width: T,
    pub sound_mgr_play: T,
    pub simple_sound_inst_for_ui_holder: T,
}

impl MN<MSig> {
    pub fn new(av: &AV, cn: &CN<Arc<CSig>>, fmv: &ForgeMV) -> Self {
        let mut mn = MN {
            base_tile_block_init: MSig {
                owner: cn.base_tile_block.clone(),
                name: cs("<init>"),
                sig: msig([cn.block_beh_props.sig.to_bytes()], b"V"),
            },
            tile_block_new_tile: MSig {
                owner: cn.tile_block.clone(),
                name: cs("m_142194_"),
                sig: msig([cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes()], cn.tile.sig.to_bytes()),
            },
            block_default_state: MSig { owner: cn.block.clone(), name: cs("m_49966_"), sig: msig([], cn.block_state.sig.to_bytes()) },
            block_beh_props_of: MSig { owner: cn.block_beh_props.clone(), name: cs("m_284310_"), sig: msig([], cn.block_beh_props.sig.to_bytes()) },
            block_beh_props_strength: MSig {
                owner: cn.block_beh_props.clone(),
                name: cs("m_60913_"),
                sig: msig([B("FF")], cn.block_beh_props.sig.to_bytes()),
            },
            block_beh_props_dyn_shape: MSig {
                owner: cn.block_beh_props.clone(),
                name: cs("m_60988_"),
                sig: msig([], cn.block_beh_props.sig.to_bytes()),
            },
            block_beh_props_sound: MSig {
                owner: cn.block_beh_props.clone(),
                name: cs("m_60918_"),
                sig: msig([cn.sound_type.sig.to_bytes()], cn.block_beh_props.sig.to_bytes()),
            },
            block_beh_get_render_shape: MSig {
                owner: cn.block_beh.clone(),
                name: cs("m_7514_"),
                sig: msig([cn.block_state.sig.to_bytes()], cn.render_shape.sig.to_bytes()),
            },
            block_beh_get_shape: MSig {
                owner: cn.block_beh.clone(),
                name: cs("m_5940_"),
                sig: msig(
                    [cn.block_state.sig.to_bytes(), cn.block_getter.sig.to_bytes(), cn.block_pos.sig.to_bytes(), cn.collision_ctx.sig.to_bytes()],
                    cn.voxel_shape.sig.to_bytes(),
                ),
            },
            block_beh_get_drops: MSig {
                owner: cn.block_beh.clone(),
                name: cs("m_49635_"),
                sig: msig([cn.block_state.sig.to_bytes(), cn.loot_builder.sig.to_bytes()], b"Ljava/util/List;"),
            },
            block_beh_on_place: MSig {
                owner: cn.block_beh.clone(),
                name: cs("m_6807_"),
                sig: msig(
                    [cn.block_state.sig.to_bytes(), cn.level.sig.to_bytes(), cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes(), b"Z"],
                    b"V",
                ),
            },
            block_beh_use: MSig {
                owner: cn.block_beh.clone(),
                name: cs(b"m_6227_"),
                sig: msig(
                    [
                        cn.block_state.sig.to_bytes(),
                        cn.level.sig.to_bytes(),
                        cn.block_pos.sig.to_bytes(),
                        cn.player.sig.to_bytes(),
                        cn.interaction_hand.sig.to_bytes(),
                        cn.block_hit_result.sig.to_bytes(),
                    ],
                    cn.interaction_result.sig.to_bytes(),
                ),
            },
            block_item_init: MSig {
                owner: cn.block_item.clone(),
                name: cs("<init>"),
                sig: msig([cn.block.sig.to_bytes(), cn.item_props.sig.to_bytes()], b"V"),
            },
            block_item_place_block: MSig {
                owner: cn.block_item.clone(),
                name: cs("m_7429_"),
                sig: msig([cn.block_place_ctx.sig.to_bytes(), cn.block_state.sig.to_bytes()], b"Z"),
            },
            block_getter_get_block_state: MSig {
                owner: cn.block_getter.clone(),
                name: cs("m_8055_"),
                sig: msig([cn.block_pos.sig.to_bytes()], cn.block_state.sig.to_bytes()),
            },
            block_getter_get_tile: MSig {
                owner: cn.block_getter.clone(),
                name: cs("m_7702_"),
                sig: msig([cn.block_pos.sig.to_bytes()], cn.tile.sig.to_bytes()),
            },
            vec3i_x: MSig { owner: cn.vec3i.clone(), name: cs("f_123285_"), sig: cs("I") },
            vec3i_y: MSig { owner: cn.vec3i.clone(), name: cs("f_123286_"), sig: cs("I") },
            vec3i_z: MSig { owner: cn.vec3i.clone(), name: cs("f_123289_"), sig: cs("I") },
            block_pos_init: MSig { owner: cn.block_pos.clone(), name: cs("<init>"), sig: cs("(III)V") },
            block_state_get_block: MSig { owner: cn.block_state.clone(), name: cs("m_60734_"), sig: msig([], cn.block.sig.to_bytes()) },
            blocks_fire: MSig { owner: cn.blocks.clone(), name: cs("f_50083_"), sig: cn.block.sig.clone() },
            tile_supplier_create: MSig {
                owner: cn.tile_supplier.clone(),
                name: cs("m_155267_"),
                sig: msig([cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes()], cn.tile.sig.to_bytes()),
            },
            tile_type_init: MSig {
                owner: cn.tile_type.clone(),
                name: cs("<init>"),
                sig: msig([cn.tile_supplier.sig.to_bytes(), b"Ljava/util/Set;", cn.dfu_type.sig.to_bytes()], b"V"),
            },
            tile_init: MSig {
                owner: cn.tile.clone(),
                name: cs("<init>"),
                sig: msig([cn.tile_type.sig.to_bytes(), cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes()], b"V"),
            },
            tile_load: MSig { owner: cn.tile.clone(), name: cs("m_142466_"), sig: msig([cn.nbt_compound.sig.to_bytes()], b"V") },
            tile_get_update_tag: MSig { owner: cn.tile.clone(), name: cs("m_5995_"), sig: msig([], cn.nbt_compound.sig.to_bytes()) },
            tile_get_update_packet: MSig { owner: cn.tile.clone(), name: cs("m_58483_"), sig: msig([], cn.packet.sig.to_bytes()) },
            tile_save_additional: MSig { owner: cn.tile.clone(), name: cs("m_183515_"), sig: msig([cn.nbt_compound.sig.to_bytes()], b"V") },
            tile_level: MSig { owner: cn.tile.clone(), name: cs("f_58857_"), sig: cn.level.sig.clone() },
            tile_pos: MSig { owner: cn.tile.clone(), name: cs("f_58858_"), sig: cn.block_pos.sig.clone() },
            sound_type_metal: MSig { owner: cn.sound_type.clone(), name: cs("f_56743_"), sig: cn.sound_type.sig.clone() },
            item_get_desc_id: MSig { owner: cn.item.clone(), name: cs("m_5524_"), sig: cs("()Ljava/lang/String;") },
            item_stack_init: MSig {
                owner: cn.item_stack.clone(),
                name: cs("<init>"),
                sig: msig([cn.item_like.sig.to_bytes(), b"I", cn.nbt_compound.sig.to_bytes()], b"V"),
            },
            creative_tab_items_gen_accept: MSig {
                owner: cn.creative_tab_items_gen.clone(),
                name: cs("m_257865_"),
                sig: msig([cn.creative_tab_params.sig.to_bytes(), cn.creative_tab_output.sig.to_bytes()], b"V"),
            },
            render_shape_tile: MSig { owner: cn.render_shape.clone(), name: cs("ENTITYBLOCK_ANIMATED"), sig: cn.render_shape.sig.clone() },
            resource_loc_init: MSig { owner: cn.resource_loc.clone(), name: cs("<init>"), sig: cs("(Ljava/lang/String;Ljava/lang/String;)V") },
            shapes_create: MSig { owner: cn.shapes.clone(), name: cs("m_166049_"), sig: msig([B("DDDDDD")], cn.voxel_shape.sig.to_bytes()) },
            s2c_tile_data_create: MSig {
                owner: cn.s2c_tile_data.clone(),
                name: cs("m_195640_"),
                sig: msig([cn.tile.sig.to_bytes()], cn.s2c_tile_data.sig.to_bytes()),
            },
            nbt_compound_init: MSig { owner: cn.nbt_compound.clone(), name: cs("<init>"), sig: cs("()V") },
            nbt_compound_put_byte_array: MSig { owner: cn.nbt_compound.clone(), name: cs("m_128382_"), sig: cs("(Ljava/lang/String;[B)V") },
            nbt_compound_get_byte_array: MSig { owner: cn.nbt_compound.clone(), name: cs("m_128463_"), sig: cs("(Ljava/lang/String;)[B") },
            use_on_ctx_get_level: MSig { owner: cn.use_on_ctx.clone(), name: cs("m_43725_"), sig: msig([], cn.level.sig.to_bytes()) },
            use_on_ctx_get_clicked_pos: MSig { owner: cn.use_on_ctx.clone(), name: cs("m_8083_"), sig: msig([], cn.block_pos.sig.to_bytes()) },
            use_on_ctx_get_clicked_face: MSig { owner: cn.use_on_ctx.clone(), name: cs("m_43719_"), sig: msig([], cn.dir.sig.to_bytes()) },
            dir_3d_data: MSig { owner: cn.dir.clone(), name: cs("f_122339_"), sig: cs("I") },
            dir_by_3d_data: MSig {
                owner: cn.dir.clone(),
                name: cs("f_122348_"),
                sig: cs(Vec::from_iter(b"[".iter().chain(cn.dir.sig.as_bytes()).copied())),
            },
            level_set_block_and_update: MSig {
                owner: cn.level.clone(),
                name: cs("m_46597_"),
                sig: msig([cn.block_pos.sig.to_bytes(), cn.block_state.sig.to_bytes()], b"Z"),
            },
            level_update_neighbors_for_out_signal: MSig {
                owner: cn.level.clone(),
                name: cs("m_46717_"),
                sig: msig([cn.block_pos.sig.to_bytes(), cn.block.sig.to_bytes()], b"V"),
            },
            level_is_client: MSig { owner: cn.level.clone(), name: cs("f_46443_"), sig: cs("Z") },
            level_get_chunk_source: MSig { owner: cn.level.clone(), name: cs("m_7726_"), sig: msig([], cn.chunk_source.sig.to_bytes()) },
            menu_provider_create_menu: MSig {
                owner: cn.menu_provider.clone(),
                name: cs("m_7208_"),
                sig: msig([b"I", cn.inventory.sig.to_bytes(), cn.player.sig.to_bytes()], cn.container_menu.sig.to_bytes()),
            },
            menu_provider_get_display_name: MSig {
                owner: cn.menu_provider.clone(),
                name: cs("m_5446_"),
                sig: msig([], cn.chat_component.sig.to_bytes()),
            },
            chat_component_translatable: MSig {
                owner: cn.chat_component.clone(),
                name: cs("m_237115_"),
                sig: msig([B("Ljava/lang/String;")], cn.chat_mutable_component.sig.to_bytes()),
            },
            chat_component_literal: MSig {
                owner: cn.chat_component.clone(),
                name: cs("m_237113_"),
                sig: msig([B("Ljava/lang/String;")], cn.chat_mutable_component.sig.to_bytes()),
            },
            chat_component_to_formatted: MSig {
                owner: cn.chat_component.clone(),
                name: cs("m_7532_"),
                sig: msig([], cn.formatted_char_seq.sig.to_bytes()),
            },
            friendly_byte_buf_read_byte_array: MSig { owner: cn.friendly_byte_buf.clone(), name: cs("m_130052_"), sig: cs("()[B") },
            friendly_byte_buf_write_byte_array: MSig {
                owner: cn.friendly_byte_buf.clone(),
                name: cs("m_130087_"),
                sig: msig([B("[B")], cn.friendly_byte_buf.sig.to_bytes()),
            },
            inventory_player: MSig { owner: cn.inventory.clone(), name: cs("f_35978_"), sig: cn.player.sig.clone() },
            entity_level: MSig { owner: cn.entity.clone(), name: cs("m_9236_"), sig: msig([], cn.level.sig.to_bytes()) },
            container_still_valid: MSig {
                owner: cn.container.clone(),
                name: cs("m_272074_"),
                sig: msig([cn.tile.sig.to_bytes(), cn.player.sig.to_bytes()], b"Z"),
            },
            player_profile: MSig { owner: cn.player.clone(), name: cs("f_36084_"), sig: cn.game_profile.sig.clone() },
            player_container_menu: MSig { owner: cn.player.clone(), name: cs("f_36096_"), sig: cn.container_menu.sig.clone() },
            game_profile_get_name: MSig { owner: cn.game_profile.clone(), name: cs("getName"), sig: cs("()Ljava/lang/String;") },
            sound_evts_ui_btn_click: MSig { owner: cn.sound_evts.clone(), name: cs("f_12490_"), sig: cn.holder_ref.sig.clone() },
            server_chunk_cache_block_changed: MSig {
                owner: cn.server_chunk_cache.clone(),
                name: cs("m_8450_"),
                sig: msig([cn.block_pos.sig.to_bytes()], b"V"),
            },
            // Client
            tile_renderer_render: MSig {
                owner: cn.tile_renderer.clone(),
                name: cs("m_6922_"),
                sig: msig([cn.tile.sig.to_bytes(), b"F", cn.pose_stack.sig.to_bytes(), cn.multi_buffer_source.sig.to_bytes(), b"II"], b"V"),
            },
            tile_renderer_provider_create: MSig {
                owner: cn.tile_renderer_provider.clone(),
                name: cs("m_173570_"),
                sig: msig([cn.tile_renderer_provider_ctx.sig.to_bytes()], cn.tile_renderer.sig.to_bytes()),
            },
            pose_pose: MSig { owner: cn.pose.clone(), name: cs("f_85852_"), sig: cn.matrix4f.sig.clone() },
            pose_stack_last: MSig { owner: cn.pose_stack.clone(), name: cs("m_85850_"), sig: msig([], cn.pose.sig.to_bytes()) },
            matrix4fc_read: MSig { owner: cn.matrix4fc.clone(), name: cs("getToAddress"), sig: msig([B("J")], cn.matrix4fc.sig.to_bytes()) },
            atlas_loc: MSig { owner: cn.atlas.clone(), name: cs("m_118330_"), sig: msig([], cn.resource_loc.sig.to_bytes()) },
            atlas_loc_blocks: MSig { owner: cn.atlas.clone(), name: cs("f_118259_"), sig: cn.resource_loc.sig.clone() },
            atlas_get_sprite: MSig {
                owner: cn.atlas.clone(),
                name: cs("m_118316_"),
                sig: msig([cn.resource_loc.sig.to_bytes()], cn.sprite.sig.to_bytes()),
            },
            sprite_u0: MSig { owner: cn.sprite.clone(), name: cs("f_118351_"), sig: cs("F") },
            sprite_v0: MSig { owner: cn.sprite.clone(), name: cs("f_118353_"), sig: cs("F") },
            sprite_u1: MSig { owner: cn.sprite.clone(), name: cs("f_118352_"), sig: cs("F") },
            sprite_v1: MSig { owner: cn.sprite.clone(), name: cs("f_118354_"), sig: cs("F") },
            sheets_solid: MSig { owner: cn.sheets.clone(), name: cs("m_110789_"), sig: msig([], cn.render_type.sig.to_bytes()) },
            buffer_source_get_buffer: MSig {
                owner: cn.multi_buffer_source.clone(),
                name: cs("m_6299_"),
                sig: msig([cn.render_type.sig.to_bytes()], cn.vertex_consumer.sig.to_bytes()),
            },
            vertex_consumer_vertex: MSig { owner: cn.vertex_consumer.clone(), name: cs("m_5954_"), sig: cs("(FFFFFFFFFIIFFF)V") },
            vertex_consumer_pos: MSig {
                owner: cn.vertex_consumer.clone(),
                name: cs("m_252986_"),
                sig: msig([cn.matrix4f.sig.to_bytes(), b"FFF"], cn.vertex_consumer.sig.to_bytes()),
            },
            vertex_consumer_color: MSig {
                owner: cn.vertex_consumer.clone(),
                name: cs("m_85950_"),
                sig: msig([B("FFFF")], cn.vertex_consumer.sig.to_bytes()),
            },
            vertex_consumer_end_vertex: MSig { owner: cn.vertex_consumer.clone(), name: cs("m_5752_"), sig: cs("()V") },
            container_menu_init: MSig { owner: cn.container_menu.clone(), name: cs("<init>"), sig: msig([cn.menu_type.sig.to_bytes(), b"I"], b"V") },
            container_menu_still_valid: MSig { owner: cn.container_menu.clone(), name: cs("m_6875_"), sig: msig([cn.player.sig.to_bytes()], b"Z") },
            container_menu_quick_move_stack: MSig {
                owner: cn.container_menu.clone(),
                name: cs("m_7648_"),
                sig: msig([cn.player.sig.to_bytes(), b"I"], cn.item_stack.sig.to_bytes()),
            },
            container_menu_id: MSig { owner: cn.container_menu.clone(), name: cs("f_38840_"), sig: cs("I") },
            menu_screens_reg: MSig {
                owner: cn.menu_screens.clone(),
                name: cs("m_96206_"),
                sig: msig([cn.menu_type.sig.to_bytes(), cn.screen_constructor.sig.to_bytes()], b"V"),
            },
            screen_constructor_create: MSig {
                owner: cn.screen_constructor.clone(),
                name: cs("m_96214_"),
                sig: msig(
                    [cn.container_menu.sig.to_bytes(), cn.inventory.sig.to_bytes(), cn.chat_component.sig.to_bytes()],
                    cn.screen.sig.to_bytes(),
                ),
            },
            screen_title: MSig { owner: cn.screen.clone(), name: cs("f_96539_"), sig: cs(cn.chat_component.sig.clone()) },
            screen_font: MSig { owner: cn.screen.clone(), name: cs("f_96547_"), sig: cs(cn.font.sig.clone()) },
            screen_width: MSig { owner: cn.screen.clone(), name: cs("f_96543_"), sig: cs("I") },
            screen_height: MSig { owner: cn.screen.clone(), name: cs("f_96544_"), sig: cs("I") },
            screen_render_background: MSig { owner: cn.screen.clone(), name: cs("m_280273_"), sig: msig([cn.gui_graphics.sig.to_bytes()], b"V") },
            container_screen_init: MSig {
                owner: cn.container_screen.clone(),
                name: cs("<init>"),
                sig: msig([cn.container_menu.sig.to_bytes(), cn.inventory.sig.to_bytes(), cn.chat_component.sig.to_bytes()], b"V"),
            },
            container_screen_minit: MSig { owner: cn.container_screen.clone(), name: cs("m_7856_"), sig: cs("()V") },
            container_screen_img_width: MSig { owner: cn.container_screen.clone(), name: cs("f_97726_"), sig: cs("I") },
            container_screen_img_height: MSig { owner: cn.container_screen.clone(), name: cs("f_97727_"), sig: cs("I") },
            container_screen_title_x: MSig { owner: cn.container_screen.clone(), name: cs("f_97728_"), sig: cs("I") },
            container_screen_title_y: MSig { owner: cn.container_screen.clone(), name: cs("f_97729_"), sig: cs("I") },
            container_screen_left: MSig { owner: cn.container_screen.clone(), name: cs("f_97735_"), sig: cs("I") },
            container_screen_top: MSig { owner: cn.container_screen.clone(), name: cs("f_97736_"), sig: cs("I") },
            container_screen_menu: MSig { owner: cn.container_screen.clone(), name: cs("f_97732_"), sig: cn.container_menu.sig.clone() },
            container_screen_render_bg: MSig {
                owner: cn.container_screen.clone(),
                name: cs("m_7286_"),
                sig: msig([cn.gui_graphics.sig.to_bytes(), b"FII"], b"V"),
            },
            container_screen_render_labels: MSig {
                owner: cn.container_screen.clone(),
                name: cs("m_280003_"),
                sig: msig([cn.gui_graphics.sig.to_bytes(), b"II"], b"V"),
            },
            container_screen_mouse_clicked: MSig { owner: cn.container_screen.clone(), name: cs("m_6375_"), sig: cs("(DDI)Z") },
            container_screen_mouse_dragged: MSig { owner: cn.container_screen.clone(), name: cs("m_7979_"), sig: cs("(DDIDD)Z") },
            container_screen_mouse_released: MSig { owner: cn.container_screen.clone(), name: cs("m_6348_"), sig: cs("(DDI)Z") },
            gui_graphics_draw_formatted: MSig {
                owner: cn.gui_graphics.clone(),
                name: cs("m_280649_"),
                sig: msig([cn.font.sig.to_bytes(), cn.formatted_char_seq.sig.to_bytes(), b"IIIZ"], b"I"),
            },
            gui_graphics_pose: MSig { owner: cn.gui_graphics.clone(), name: cs("f_279612_"), sig: cn.pose_stack.sig.clone() },
            render_sys_set_shader: MSig { owner: cn.render_sys.clone(), name: cs(b"setShader"), sig: cs("(Ljava/util/function/Supplier;)V") },
            render_sys_enable_blend: MSig { owner: cn.render_sys.clone(), name: cs(b"enableBlend"), sig: cs("()V") },
            render_sys_enable_cull: MSig { owner: cn.render_sys.clone(), name: cs(b"enableCull"), sig: cs("()V") },
            render_sys_disable_blend: MSig { owner: cn.render_sys.clone(), name: cs(b"disableBlend"), sig: cs("()V") },
            render_sys_disable_cull: MSig { owner: cn.render_sys.clone(), name: cs(b"disableCull"), sig: cs("()V") },
            game_renderer_get_pos_color_shader: MSig {
                owner: cn.game_renderer.clone(),
                name: cs(b"m_172811_"),
                sig: msig([], cn.shader_inst.sig.to_bytes()),
            },
            tesselator_get_inst: MSig { owner: cn.tesselator.clone(), name: cs(b"m_85913_"), sig: msig([], cn.tesselator.sig.to_bytes()) },
            tesselator_get_builder: MSig { owner: cn.tesselator.clone(), name: cs(b"m_85915_"), sig: msig([], cn.buffer_builder.sig.to_bytes()) },
            tesselator_end: MSig { owner: cn.tesselator.clone(), name: cs(b"m_85914_"), sig: cs("()V") },
            vertex_mode_tris: MSig { owner: cn.vertex_mode.clone(), name: cs(b"TRIANGLES"), sig: cn.vertex_mode.sig.clone() },
            default_vertex_fmt_pos_color: MSig { owner: cn.default_vertex_fmt.clone(), name: cs(b"f_85815_"), sig: cn.vertex_fmt.sig.clone() },
            buffer_builder_begin: MSig {
                owner: cn.buffer_builder.clone(),
                name: cs(b"m_166779_"),
                sig: msig([cn.vertex_mode.sig.to_bytes(), cn.vertex_fmt.sig.to_bytes()], b"V"),
            },
            mc_get_inst: MSig { owner: cn.mc.clone(), name: cs("m_91087_"), sig: msig([], cn.mc.sig.to_bytes()) },
            mc_get_window: MSig { owner: cn.mc.clone(), name: cs("m_91268_"), sig: msig([], cn.window.sig.to_bytes()) },
            mc_get_sound_mgr: MSig { owner: cn.mc.clone(), name: cs("m_91106_"), sig: msig([], cn.sound_mgr.sig.to_bytes()) },
            window_get_gui_scale: MSig { owner: cn.window.clone(), name: cs("m_85449_"), sig: cs("()D") },
            font_width: MSig { owner: cn.font.clone(), name: cs("m_92724_"), sig: msig([cn.formatted_char_seq.sig.to_bytes()], b"I") },
            sound_mgr_play: MSig { owner: cn.sound_mgr.clone(), name: cs("m_120367_"), sig: msig([cn.sound_inst.sig.to_bytes()], b"V") },
            simple_sound_inst_for_ui_holder: MSig {
                owner: cn.simple_sound_inst.clone(),
                name: cs("m_263171_"),
                sig: msig([cn.holder.sig.to_bytes(), b"F"], cn.simple_sound_inst.sig.to_bytes()),
            },
        };
        if !fmv.fml_naming_is_srg {
            let from = av.ldr.jni.new_utf(c"srg").unwrap();
            let mapper = fmv.cml_inst.call_object_method(fmv.cml_get_mapper, &[from.raw]).unwrap().unwrap().opt_get(&av.jv).unwrap().unwrap();
            mn.apply(|x| {
                let name = mapper.jni.new_utf(&x.name).unwrap();
                let name = mapper.bifunc_apply(&av.jv, if x.is_method() { fmv.naming_domain_m.raw } else { fmv.naming_domain_f.raw }, name.raw);
                x.name = cs(name.unwrap().unwrap().utf_chars().unwrap().to_vec())
            })
        }
        mn
    }
}

pub struct MV {
    pub base_tile_block_init: usize,
    pub block_default_state: usize,
    pub block_beh_props: GlobalRef<'static>,
    pub block_beh_props_of: usize,
    pub block_beh_props_strength: usize,
    pub block_beh_props_dyn_shape: usize,
    pub block_beh_props_sound: usize,
    pub block_item: GlobalRef<'static>,
    pub block_item_init: usize,
    pub block_item_place_block: usize,
    pub block_getter_get_block_state: usize,
    pub block_getter_get_tile: usize,
    pub vec3i_x: usize,
    pub vec3i_y: usize,
    pub vec3i_z: usize,
    pub block_pos: GlobalRef<'static>,
    pub block_pos_init: usize,
    pub block_state_get_block: usize,
    pub blocks_fire: GlobalRef<'static>,
    pub tile_type: GlobalRef<'static>,
    pub tile_type_init: usize,
    pub tile: GlobalRef<'static>,
    pub tile_init: usize,
    pub tile_load: usize,
    pub tile_save_additional: usize,
    pub tile_level: usize,
    pub tile_pos: usize,
    pub sound_type_metal: GlobalRef<'static>,
    pub item: GlobalRef<'static>,
    pub item_get_desc_id: usize,
    pub item_stack: GlobalRef<'static>,
    pub item_stack_init: usize,
    pub render_shape_tile: GlobalRef<'static>,
    pub resource_loc: GlobalRef<'static>,
    pub resource_loc_init: usize,
    pub shapes: GlobalRef<'static>,
    pub shapes_create: usize,
    pub nbt_compound: GlobalRef<'static>,
    pub nbt_compound_init: usize,
    pub nbt_compound_put_byte_array: usize,
    pub nbt_compound_get_byte_array: usize,
    pub use_on_ctx_get_level: usize,
    pub use_on_ctx_get_clicked_pos: usize,
    pub use_on_ctx_get_clicked_face: usize,
    pub dir_3d_data: usize,
    pub dir_by_3d_data: GlobalRef<'static>,
    pub level_set_block_and_update: usize,
    pub level_update_neighbors_for_out_signal: usize,
    pub level_is_client: usize,
    pub level_get_chunk_source: usize,
    pub friendly_byte_buf_read_byte_array: usize,
    pub friendly_byte_buf_write_byte_array: usize,
    pub container_menu_init: usize,
    pub container_menu_id: usize,
    pub chat_component: GlobalRef<'static>,
    pub chat_component_translatable: usize,
    pub chat_component_literal: usize,
    pub chat_component_to_formatted: usize,
    pub interaction_result_pass: GlobalRef<'static>,
    pub interaction_result_success: GlobalRef<'static>,
    pub interaction_result_consume: GlobalRef<'static>,
    pub server_player: GlobalRef<'static>,
    pub inventory_player: usize,
    pub entity_level: usize,
    pub container: GlobalRef<'static>,
    pub container_still_valid: usize,
    pub player_profile: usize,
    pub player_container_menu: usize,
    pub game_profile_get_name: usize,
    pub sound_evts_ui_btn_click: GlobalRef<'static>,
    pub server_chunk_cache_block_changed: usize,
    pub client: Option<MVC>,
}

pub struct MVC {
    pub pose_pose: usize,
    pub pose_stack_last: usize,
    pub matrix4fc_read: usize,
    pub atlas_loc: usize,
    pub atlas_loc_blocks: GlobalRef<'static>,
    pub atlas_get_sprite: usize,
    pub sprite_u0: usize,
    pub sprite_v0: usize,
    pub sprite_u1: usize,
    pub sprite_v1: usize,
    pub buffer_source_get_buffer: usize,
    pub vertex_consumer_vertex: usize,
    pub vertex_consumer_pos: usize,
    pub vertex_consumer_color: usize,
    pub vertex_consumer_end_vertex: usize,
    pub menu_screens: GlobalRef<'static>,
    pub menu_screens_reg: usize,
    pub screen_title: usize,
    pub screen_font: usize,
    pub screen_width: usize,
    pub screen_height: usize,
    pub screen_render_background: usize,
    pub container_screen: GlobalRef<'static>,
    pub container_screen_init: usize,
    pub container_screen_img_width: usize,
    pub container_screen_img_height: usize,
    pub container_screen_title_x: usize,
    pub container_screen_title_y: usize,
    pub container_screen_left: usize,
    pub container_screen_top: usize,
    pub container_screen_menu: usize,
    pub container_screen_mouse_clicked: usize,
    pub container_screen_mouse_dragged: usize,
    pub container_screen_mouse_released: usize,
    pub gui_graphics_draw_formatted: usize,
    pub gui_graphics_pose: usize,
    pub render_sys: GlobalRef<'static>,
    pub render_sys_set_shader: usize,
    pub render_sys_enable_blend: usize,
    pub render_sys_enable_cull: usize,
    pub render_sys_disable_blend: usize,
    pub render_sys_disable_cull: usize,
    pub tesselator: GlobalRef<'static>,
    pub tesselator_get_inst: usize,
    pub tesselator_get_builder: usize,
    pub tesselator_end: usize,
    pub vertex_mode_tris: GlobalRef<'static>,
    pub default_vertex_fmt_pos_color: GlobalRef<'static>,
    pub buffer_builder_begin: usize,
    pub game_renderer: GlobalRef<'static>,
    pub game_renderer_get_pos_color_shader: usize,
    pub window_inst: GlobalRef<'static>,
    pub window_get_gui_scale: usize,
    pub font_width: usize,
    pub mc_inst: GlobalRef<'static>,
    pub mc_get_sound_mgr: usize,
    pub sound_mgr_play: usize,
    pub simple_sound_inst: GlobalRef<'static>,
    pub simple_sound_inst_for_ui_holder: usize,
}

impl MV {
    pub fn new(av: &AV<'static>, cn: &CN<Arc<CSig>>, mn: &MN<MSig>, is_client: bool) -> MV {
        let load = |csig: &Arc<CSig>| av.ldr.load_class(&av.jv, &csig.dot).unwrap().new_global_ref().unwrap();
        let base_tile_block = load(&cn.base_tile_block);
        let block = load(&cn.block);
        let block_beh_props = load(&cn.block_beh_props);
        let block_item = load(&cn.block_item);
        let block_getter = load(&cn.block_getter);
        let vec3i = load(&cn.vec3i);
        let block_pos = load(&cn.block_pos);
        let block_state = load(&cn.block_state);
        let tile_type = load(&cn.tile_type);
        let tile = load(&cn.tile);
        let sound_type = load(&cn.sound_type);
        let item = load(&cn.item);
        let item_stack = load(&cn.item_stack);
        let render_shape = load(&cn.render_shape);
        let resource_loc = load(&cn.resource_loc);
        let shapes = load(&cn.shapes);
        let nbt_compound = load(&cn.nbt_compound);
        let use_on_ctx = load(&cn.use_on_ctx);
        let dir = load(&cn.dir);
        let level = load(&cn.level);
        let friendly_byte_buf = load(&cn.friendly_byte_buf);
        let container_menu = load(&cn.container_menu);
        let chat_component = load(&cn.chat_component);
        let interaction_result = load(&cn.interaction_result);
        let container = load(&cn.container);
        let player = load(&cn.player);
        MV {
            base_tile_block_init: mn.base_tile_block_init.get_method_id(&base_tile_block).unwrap(),
            block_default_state: mn.block_default_state.get_method_id(&block).unwrap(),
            block_beh_props_of: mn.block_beh_props_of.get_static_method_id(&block_beh_props).unwrap(),
            block_beh_props_strength: mn.block_beh_props_strength.get_method_id(&block_beh_props).unwrap(),
            block_beh_props_dyn_shape: mn.block_beh_props_dyn_shape.get_method_id(&block_beh_props).unwrap(),
            block_beh_props_sound: mn.block_beh_props_sound.get_method_id(&block_beh_props).unwrap(),
            block_beh_props,
            block_item_init: mn.block_item_init.get_method_id(&block_item).unwrap(),
            block_item_place_block: mn.block_item_place_block.get_method_id(&block_item).unwrap(),
            block_item,
            block_getter_get_block_state: mn.block_getter_get_block_state.get_method_id(&block_getter).unwrap(),
            block_getter_get_tile: mn.block_getter_get_tile.get_method_id(&block_getter).unwrap(),
            vec3i_x: mn.vec3i_x.get_field_id(&vec3i).unwrap(),
            vec3i_y: mn.vec3i_y.get_field_id(&vec3i).unwrap(),
            vec3i_z: mn.vec3i_z.get_field_id(&vec3i).unwrap(),
            block_pos_init: mn.block_pos_init.get_method_id(&block_pos).unwrap(),
            block_pos,
            block_state_get_block: mn.block_state_get_block.get_method_id(&block_state).unwrap(),
            blocks_fire: load(&cn.blocks).static_field_2(&mn.blocks_fire),
            tile_type_init: mn.tile_type_init.get_method_id(&tile_type).unwrap(),
            tile_type,
            tile_init: mn.tile_init.get_method_id(&tile).unwrap(),
            tile_load: mn.tile_load.get_method_id(&tile).unwrap(),
            tile_save_additional: mn.tile_save_additional.get_method_id(&tile).unwrap(),
            tile_level: mn.tile_level.get_field_id(&tile).unwrap(),
            tile_pos: mn.tile_pos.get_field_id(&tile).unwrap(),
            tile,
            sound_type_metal: sound_type.static_field_2(&mn.sound_type_metal),
            item_get_desc_id: mn.item_get_desc_id.get_method_id(&item).unwrap(),
            item,
            item_stack_init: mn.item_stack_init.get_method_id(&item_stack).unwrap(),
            item_stack,
            render_shape_tile: render_shape.static_field_2(&mn.render_shape_tile),
            resource_loc_init: mn.resource_loc_init.get_method_id(&resource_loc).unwrap(),
            resource_loc,
            shapes_create: mn.shapes_create.get_static_method_id(&shapes).unwrap(),
            shapes,
            nbt_compound_init: mn.nbt_compound_init.get_method_id(&nbt_compound).unwrap(),
            nbt_compound_put_byte_array: mn.nbt_compound_put_byte_array.get_method_id(&nbt_compound).unwrap(),
            nbt_compound_get_byte_array: mn.nbt_compound_get_byte_array.get_method_id(&nbt_compound).unwrap(),
            nbt_compound,
            use_on_ctx_get_level: mn.use_on_ctx_get_level.get_method_id(&use_on_ctx).unwrap(),
            use_on_ctx_get_clicked_pos: mn.use_on_ctx_get_clicked_pos.get_method_id(&use_on_ctx).unwrap(),
            use_on_ctx_get_clicked_face: mn.use_on_ctx_get_clicked_face.get_method_id(&use_on_ctx).unwrap(),
            dir_3d_data: mn.dir_3d_data.get_field_id(&dir).unwrap(),
            dir_by_3d_data: dir.static_field_2(&mn.dir_by_3d_data),
            level_set_block_and_update: mn.level_set_block_and_update.get_method_id(&level).unwrap(),
            level_update_neighbors_for_out_signal: mn.level_update_neighbors_for_out_signal.get_method_id(&level).unwrap(),
            level_is_client: mn.level_is_client.get_field_id(&level).unwrap(),
            level_get_chunk_source: mn.level_get_chunk_source.get_method_id(&level).unwrap(),
            friendly_byte_buf_read_byte_array: mn.friendly_byte_buf_read_byte_array.get_method_id(&friendly_byte_buf).unwrap(),
            friendly_byte_buf_write_byte_array: mn.friendly_byte_buf_write_byte_array.get_method_id(&friendly_byte_buf).unwrap(),
            container_menu_init: mn.container_menu_init.get_method_id(&container_menu).unwrap(),
            container_menu_id: mn.container_menu_id.get_field_id(&container_menu).unwrap(),
            chat_component_translatable: mn.chat_component_translatable.get_static_method_id(&chat_component).unwrap(),
            chat_component_literal: mn.chat_component_literal.get_static_method_id(&chat_component).unwrap(),
            chat_component_to_formatted: mn.chat_component_to_formatted.get_method_id(&chat_component).unwrap(),
            chat_component,
            interaction_result_pass: interaction_result.static_field_1(c"PASS", &cn.interaction_result.sig),
            interaction_result_success: interaction_result.static_field_1(c"SUCCESS", &cn.interaction_result.sig),
            interaction_result_consume: interaction_result.static_field_1(c"CONSUME", &cn.interaction_result.sig),
            server_player: load(&cn.server_player),
            inventory_player: mn.inventory_player.get_field_id(&load(&cn.inventory)).unwrap(),
            entity_level: mn.entity_level.get_method_id(&load(&cn.entity)).unwrap(),
            container_still_valid: mn.container_still_valid.get_static_method_id(&container).unwrap(),
            container,
            player_profile: mn.player_profile.get_field_id(&player).unwrap(),
            player_container_menu: mn.player_container_menu.get_field_id(&player).unwrap(),
            game_profile_get_name: mn.game_profile_get_name.get_method_id(&load(&cn.game_profile)).unwrap(),
            sound_evts_ui_btn_click: load(&cn.sound_evts).static_field_2(&mn.sound_evts_ui_btn_click),
            server_chunk_cache_block_changed: mn.server_chunk_cache_block_changed.get_method_id(&load(&cn.server_chunk_cache)).unwrap(),
            client: is_client.then(|| {
                let pose = load(&cn.pose);
                let pose_stack = load(&cn.pose_stack);
                let matrix4fc = load(&cn.matrix4fc);
                let atlas = load(&cn.atlas);
                let sprite = load(&cn.sprite);
                let buffer_source = load(&cn.multi_buffer_source);
                let vertex_consumer = load(&cn.vertex_consumer);
                let menu_screens = load(&cn.menu_screens);
                let screen = load(&cn.screen);
                let container_screen = load(&cn.container_screen);
                let gui_graphics = load(&cn.gui_graphics);
                let render_sys = load(&cn.render_sys);
                let tesselator = load(&cn.tesselator);
                let buffer_builder = load(&cn.buffer_builder);
                let vertex_mode = load(&cn.vertex_mode);
                let default_vertex_fmt = load(&cn.default_vertex_fmt);
                let game_renderer = load(&cn.game_renderer);
                let mc = load(&cn.mc);
                let mc_inst = mc.call_static_object_method(mn.mc_get_inst.get_static_method_id(&mc).unwrap(), &[]).unwrap().unwrap();
                let window_inst = mc_inst.call_object_method(mn.mc_get_window.get_method_id(&mc).unwrap(), &[]).unwrap().unwrap();
                let window = window_inst.get_object_class();
                let simple_sound_inst = load(&cn.simple_sound_inst);
                MVC {
                    pose_pose: mn.pose_pose.get_field_id(&pose).unwrap(),
                    pose_stack_last: mn.pose_stack_last.get_method_id(&pose_stack).unwrap(),
                    matrix4fc_read: mn.matrix4fc_read.get_method_id(&matrix4fc).unwrap(),
                    atlas_loc: mn.atlas_loc.get_method_id(&atlas).unwrap(),
                    atlas_loc_blocks: atlas.static_field_2(&mn.atlas_loc_blocks),
                    atlas_get_sprite: mn.atlas_get_sprite.get_method_id(&atlas).unwrap(),
                    sprite_u0: mn.sprite_u0.get_field_id(&sprite).unwrap(),
                    sprite_v0: mn.sprite_v0.get_field_id(&sprite).unwrap(),
                    sprite_u1: mn.sprite_u1.get_field_id(&sprite).unwrap(),
                    sprite_v1: mn.sprite_v1.get_field_id(&sprite).unwrap(),
                    buffer_source_get_buffer: mn.buffer_source_get_buffer.get_method_id(&buffer_source).unwrap(),
                    vertex_consumer_vertex: mn.vertex_consumer_vertex.get_method_id(&vertex_consumer).unwrap(),
                    vertex_consumer_pos: mn.vertex_consumer_pos.get_method_id(&vertex_consumer).unwrap(),
                    vertex_consumer_color: mn.vertex_consumer_color.get_method_id(&vertex_consumer).unwrap(),
                    vertex_consumer_end_vertex: mn.vertex_consumer_end_vertex.get_method_id(&vertex_consumer).unwrap(),
                    menu_screens_reg: mn.menu_screens_reg.get_static_method_id(&menu_screens).unwrap(),
                    menu_screens,
                    screen_title: mn.screen_title.get_field_id(&screen).unwrap(),
                    screen_font: mn.screen_font.get_field_id(&screen).unwrap(),
                    screen_width: mn.screen_width.get_field_id(&screen).unwrap(),
                    screen_height: mn.screen_height.get_field_id(&screen).unwrap(),
                    screen_render_background: mn.screen_render_background.get_method_id(&screen).unwrap(),
                    container_screen_init: mn.container_screen_init.get_method_id(&container_screen).unwrap(),
                    container_screen_img_width: mn.container_screen_img_width.get_field_id(&container_screen).unwrap(),
                    container_screen_img_height: mn.container_screen_img_height.get_field_id(&container_screen).unwrap(),
                    container_screen_title_x: mn.container_screen_title_x.get_field_id(&container_screen).unwrap(),
                    container_screen_title_y: mn.container_screen_title_y.get_field_id(&container_screen).unwrap(),
                    container_screen_left: mn.container_screen_left.get_field_id(&container_screen).unwrap(),
                    container_screen_top: mn.container_screen_top.get_field_id(&container_screen).unwrap(),
                    container_screen_menu: mn.container_screen_menu.get_field_id(&container_screen).unwrap(),
                    container_screen_mouse_clicked: mn.container_screen_mouse_clicked.get_method_id(&container_screen).unwrap(),
                    container_screen_mouse_dragged: mn.container_screen_mouse_dragged.get_method_id(&container_screen).unwrap(),
                    container_screen_mouse_released: mn.container_screen_mouse_released.get_method_id(&container_screen).unwrap(),
                    container_screen,
                    gui_graphics_draw_formatted: mn.gui_graphics_draw_formatted.get_method_id(&gui_graphics).unwrap(),
                    gui_graphics_pose: mn.gui_graphics_pose.get_field_id(&gui_graphics).unwrap(),
                    render_sys_set_shader: mn.render_sys_set_shader.get_static_method_id(&render_sys).unwrap(),
                    render_sys_enable_blend: mn.render_sys_enable_blend.get_static_method_id(&render_sys).unwrap(),
                    render_sys_enable_cull: mn.render_sys_enable_cull.get_static_method_id(&render_sys).unwrap(),
                    render_sys_disable_blend: mn.render_sys_disable_blend.get_static_method_id(&render_sys).unwrap(),
                    render_sys_disable_cull: mn.render_sys_disable_cull.get_static_method_id(&render_sys).unwrap(),
                    render_sys,
                    tesselator_get_inst: mn.tesselator_get_inst.get_static_method_id(&tesselator).unwrap(),
                    tesselator_get_builder: mn.tesselator_get_builder.get_method_id(&tesselator).unwrap(),
                    tesselator_end: mn.tesselator_end.get_method_id(&tesselator).unwrap(),
                    tesselator,
                    vertex_mode_tris: vertex_mode.static_field_2(&mn.vertex_mode_tris),
                    default_vertex_fmt_pos_color: default_vertex_fmt.static_field_2(&mn.default_vertex_fmt_pos_color),
                    buffer_builder_begin: mn.buffer_builder_begin.get_method_id(&buffer_builder).unwrap(),
                    game_renderer_get_pos_color_shader: mn.game_renderer_get_pos_color_shader.get_static_method_id(&game_renderer).unwrap(),
                    game_renderer,
                    window_get_gui_scale: mn.window_get_gui_scale.get_method_id(&window).unwrap(),
                    window_inst: window_inst.new_global_ref().unwrap(),
                    font_width: mn.font_width.get_method_id(&load(&cn.font)).unwrap(),
                    mc_get_sound_mgr: mn.mc_get_sound_mgr.get_method_id(&mc).unwrap(),
                    mc_inst: mc_inst.new_global_ref().unwrap(),
                    sound_mgr_play: mn.sound_mgr_play.get_method_id(&load(&cn.sound_mgr)).unwrap(),
                    simple_sound_inst_for_ui_holder: mn.simple_sound_inst_for_ui_holder.get_static_method_id(&simple_sound_inst).unwrap(),
                    simple_sound_inst,
                }
            }),
        }
    }
}

#[derive(Functor)]
pub struct GregCN<T> {
    pub reg: T,
    pub item_builder: T,
    pub non_null_fn: T,
    pub creative_tab_items_gen: T,
    pub values: T,
    pub caps: T,
    pub dyn_resource_pack: T,
    pub material_block_renderer: T,
    pub energy_container: T,
    pub pipe_block: T,
    pub pipe_node: T,
}

impl GregCN<Arc<CSig>> {
    pub fn new() -> Self {
        let names = GregCN::<&[u8]> {
            reg: b"com.gregtechceu.gtceu.api.registry.registrate.GTRegistrate",
            item_builder: b"com.tterrag.registrate.builders.ItemBuilder",
            non_null_fn: b"com.tterrag.registrate.util.nullness.NonNullFunction",
            creative_tab_items_gen: b"com.gregtechceu.gtceu.common.data.GTCreativeModeTabs$RegistrateDisplayItemsGenerator",
            values: b"com.gregtechceu.gtceu.api.GTValues",
            caps: b"com.gregtechceu.gtceu.api.capability.forge.GTCapability",
            dyn_resource_pack: b"com.gregtechceu.gtceu.data.pack.GTDynamicResourcePack",
            material_block_renderer: b"com.gregtechceu.gtceu.client.renderer.block.MaterialBlockRenderer",
            energy_container: b"com.gregtechceu.gtceu.api.capability.IEnergyContainer",
            pipe_block: b"com.gregtechceu.gtceu.api.block.PipeBlock",
            pipe_node: b"com.gregtechceu.gtceu.api.pipenet.IPipeNode",
        };
        names.fmap(|x| Arc::new(CSig::new(x)))
    }
}

pub struct GregMN {
    pub reg_item: MSig,
    pub can_input_eu_from_side: MSig,
    pub accept_eu: MSig,
    pub change_eu: MSig,
    pub get_eu_stored: MSig,
    pub get_eu_capacity: MSig,
    pub get_input_amps: MSig,
    pub get_input_volts: MSig,
    pub get_input_eu_per_sec: MSig,
    pub pipe_block_get_node: MSig,
    pub pipe_block_can_connect: MSig,
    pub pipe_node_set_connection: MSig,
}

impl GregMN {
    pub fn new(cn: &CN<Arc<CSig>>, gcn: &GregCN<Arc<CSig>>) -> Self {
        GregMN {
            reg_item: MSig {
                owner: gcn.reg.clone(),
                name: cs("item"),
                sig: msig([b"Ljava/lang/String;", gcn.non_null_fn.sig.to_bytes()], gcn.item_builder.sig.to_bytes()),
            },
            can_input_eu_from_side: MSig { owner: gcn.energy_container.clone(), name: cs("inputsEnergy"), sig: msig([cn.dir.sig.to_bytes()], b"Z") },
            accept_eu: MSig {
                owner: gcn.energy_container.clone(),
                name: cs("acceptEnergyFromNetwork"),
                sig: msig([cn.dir.sig.to_bytes(), b"JJ"], b"J"),
            },
            change_eu: MSig { owner: gcn.energy_container.clone(), name: cs("changeEnergy"), sig: cs("(J)J") },
            get_eu_stored: MSig { owner: gcn.energy_container.clone(), name: cs("getEnergyStored"), sig: cs("()J") },
            get_eu_capacity: MSig { owner: gcn.energy_container.clone(), name: cs("getEnergyCapacity"), sig: cs("()J") },
            get_input_amps: MSig { owner: gcn.energy_container.clone(), name: cs("getInputAmperage"), sig: cs("()J") },
            get_input_volts: MSig { owner: gcn.energy_container.clone(), name: cs("getInputVoltage"), sig: cs("()J") },
            get_input_eu_per_sec: MSig { owner: gcn.energy_container.clone(), name: cs("getInputPerSec"), sig: cs("()J") },
            pipe_block_get_node: MSig {
                owner: gcn.pipe_block.clone(),
                name: cs("getPipeTile"),
                sig: msig([cn.block_getter.sig.to_bytes(), cn.block_pos.sig.to_bytes()], gcn.pipe_node.sig.to_bytes()),
            },
            pipe_block_can_connect: MSig {
                owner: gcn.pipe_block.clone(),
                name: cs("canPipeConnectToBlock"),
                sig: msig([gcn.pipe_node.sig.to_bytes(), cn.dir.sig.to_bytes(), cn.tile.sig.to_bytes()], b"Z"),
            },
            pipe_node_set_connection: MSig {
                owner: gcn.pipe_node.clone(),
                name: cs("setConnection"),
                sig: msig([cn.dir.sig.to_bytes(), b"ZZ"], b"V"),
            },
        }
    }
}

pub struct GregMV {
    pub tier_names: GlobalRef<'static>,
    pub tier_volts: GlobalRef<'static>,
    pub dyn_resource_pack_data: GlobalRef<'static>,
    pub energy_container_cap: GlobalRef<'static>,
    pub pipe_block: GlobalRef<'static>,
    pub pipe_block_get_node: usize,
    pub pipe_block_can_connect: usize,
    pub pipe_node_set_connection: usize,
}

impl GregMV {
    pub fn new(jni: &'static JNI) -> Self {
        let GlobalObjs { av, fcn, gcn, gmn, .. } = objs();
        let load = |csig: &Arc<CSig>| av.ldr.with_jni(jni).load_class(&av.jv, &csig.dot).unwrap().new_global_ref().unwrap();
        let values = load(&gcn.values);
        let pipe_block = load(&gcn.pipe_block);
        Self {
            tier_names: values.static_field_1(c"VN", c"[Ljava/lang/String;"),
            tier_volts: values.static_field_1(c"V", c"[J"),
            dyn_resource_pack_data: load(&gcn.dyn_resource_pack).static_field_1(c"DATA", c"Ljava/util/concurrent/ConcurrentMap;"),
            energy_container_cap: load(&gcn.caps).static_field_1(c"CAPABILITY_ENERGY_CONTAINER", &fcn.cap.sig),
            pipe_block_get_node: gmn.pipe_block_get_node.get_method_id(&pipe_block).unwrap(),
            pipe_block_can_connect: gmn.pipe_block_can_connect.get_method_id(&pipe_block).unwrap(),
            pipe_node_set_connection: gmn.pipe_node_set_connection.get_method_id(&load(&gcn.pipe_node)).unwrap(),
            pipe_block,
        }
    }
}
